use std::time::Duration;
use tokio::net::UdpSocket;
use std::fs;
use bytes::BytesMut;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
mod msg;

use msg::Codec;

// Include the `items` module, which is generated from items.proto.
// It is important to maintain the same structure as in the proto.
pub mod tftp2 {
  pub mod spec {
      include!(concat!(env!("OUT_DIR"), "/tftp2.rs"));
  }
}

use tftp2::spec;

#[derive(Debug)]
pub enum Status {
  OK,
  Timeout,
  MaxRetriesExceeded,
  Error(u16),
  Unknown
}

#[derive(Debug)]
struct Sessao {
  sock: UdpSocket,
  server: SocketAddr,
  tid: bool,
  buffer: BytesMut,
  seqno: u16,
  timeout: u16,
  retries: u16,
  max_retries: u16,
  estado: Estado,
  status: Status
}

#[derive(Debug)]
#[derive(PartialEq)]
#[derive(Eq)]
#[derive(Hash)]
enum Evento {
  Msg(Vec<u8>),
  Timeout,
  Nada
}

#[derive(Debug)]
#[derive(PartialEq)]
enum Estado {
  Idle,
  RX,
  InitTX,
  TX,
  FinishTX,
  Finish
}

/// A Session is responsible for a file transfer (TX or RX).
/// When RXing, the file contents will be stored in attribute "buffer"
/// When TXing, file contents are first stored in "buffer", and then sent from there
/// In the end, attribute "status" contains status of transmission (see enum Status)
impl Sessao {
  async fn new(server:&str, port:u16, timeout: u16, retries: u16) -> Option<Self> {
    if let Some(ip) = Sessao::parse_ip(server) {
      return Some(Sessao {
        sock: UdpSocket::bind("0.0.0.0:0").await.expect("ao criar socket UDP"),
        server: SocketAddr::new(IpAddr::V4(ip), port),
        tid: false,
        buffer: BytesMut::new(),
        seqno: 1,
        timeout: timeout,
        retries: 0,
        max_retries: retries,
        estado: Estado::Idle,
        status: Status::OK
      });
    }
    None
  }

  /// converts a string with an IP adress to a Ipv4Addr
  /// is there a simpler way ???
  fn parse_ip(ip: &str) -> Option<Ipv4Addr> {
    let ip:Vec<_> = ip.split('.')
                           .map(|c| c.parse::<u8>())
                           .collect();
    if ! ip.iter().any(|x| x.is_err()) {
      if ip.len() == 4 {
        let ip:Vec<_> = ip.into_iter()
                          .map(|x| x.unwrap())
                          .collect();
        return Some(Ipv4Addr::new(ip[0], ip[1], ip[2], ip[3]));
      }
    }
    None
  }

  /// just checks if FSM is finished
  fn is_finished(&self) -> bool {
    self.estado == Estado::Finish
  }

  /// runs the FSM until it finishes
  /// waits for events and handles them
  async fn run(&mut self) {
    while ! self.is_finished() {
        let ev = self.get_event().await;
        match self.estado {
          Estado::RX => {
            self.handle_rx(ev).await;
          }
          Estado::InitTX => {
            self.handle_init_tx(ev).await;
          }
          Estado::TX => {
            self.handle_tx(ev).await;
          }
          Estado::FinishTX => {
            self.handle_finish_tx(ev).await;
          }
          Estado::Finish => {
          }
          _ => {} // Idle 
        }       
    }
  }

  /// starts transmission of a file: sends contents of "data" to a file named "fname"
  async fn send(&mut self, fname: &str, data: &[u8]) {
    if self.estado != Estado::Idle {
        panic!("sessão em uso");
    }
    
    if let Some(req) = msg::Requisicao::new_wrq(fname, msg::Modo::Octet) {
      let mesg = req.serialize();
      println!("wrq: {:?}", mesg);
      if let Ok(_n) = self.sock.send_to(&mesg, self.server).await {
        self.buffer.extend_from_slice(data);
        self.estado = Estado::InitTX;
        self.run().await;
      } else {
        self.estado = Estado::Finish;
        self.status = Status::Unknown;
      }
    }
  }

  /// starts reception of file "fname".
  /// In the end, its contents are contained in attribute "buffer"
  async fn receive(&mut self, fname: &str) -> Option<()>{
    if self.estado != Estado::Idle {
        panic!("sessão em uso");
    }
    if let Some(req) = msg::Requisicao::new_rrq(fname, msg::Modo::Octet) {
        let mesg = req.serialize();
        println!("rrq: {:?}", mesg);
        if let Ok(_n) = self.sock.send_to(&mesg, self.server).await {
          self.estado = Estado::RX;
          self.run().await;
          return Some(());
        }
    }
    self.estado = Estado::Finish;
    self.status = Status::Unknown;
    None        
  }

  /// waits for an event, and return it
  async fn get_event(&mut self) -> Evento {
    let mut buf = [0; 1024];
    let f_timeout = tokio::time::sleep(Duration::from_secs(self.timeout as u64));

    tokio::select! {
      val = f_timeout, if self.timeout > 0 => {
        println!("Timeout: val={:?}", val);
        return Evento::Timeout;
      }
      val = self.sock.recv_from(&mut buf) => {
        if let Ok((len,addr)) = val {
          println!("Rx: val={:?}", (len,addr));
          // TODO: conferir o ip do servidor
          if ! self.tid {
            self.tid = true;
            self.server.set_port(addr.port());
          }
          if self.server == addr {
            let msg = buf[..len].to_vec();
            return Evento::Msg(msg);
          }
        }
      }
    }
    Evento::Nada
  }
  
  /// FSM handler for state RX
  async fn handle_rx(&mut self, ev: Evento) {
    println!("rx");

    match ev {
        Evento::Timeout => {
            self.estado = Estado::Finish;
            self.status = Status::Timeout;
        }
        Evento::Msg(buffer) => {
            if let Some(mesg) = msg::from_bytes(buffer) {
                match mesg {
                    msg::Mensagem::Data(data) => {
                        if data.block == self.seqno {
                            self.seqno += 1;
                            self.buffer.extend_from_slice(&data.body);
                            if data.body.len() < msg::DATA::SIZE {
                                self.estado = Estado::Finish;
                            }
                        }
                        if let Some(resp) = msg::ACK::new(data.block) {
                            let mesg = resp.serialize();                                
                            if let Err(_e) = self.sock.send_to(&mesg, self.server).await {
                              self.estado = Estado::Finish;
                              self.status = Status::Unknown;
                            }
                        }                        
                    }
                    msg::Mensagem::Err(err) => {
                        self.estado = Estado::Finish;
                        self.status = Status::Error(err.err_code);

                    }
                    _ => {

                    }
                }
            }
        }
        _ => {
          println!("Alguma outra coisa ...");
        }
    }    
    println!("handle_rx: terminou");
  }

  /// calculates current chunk size
  fn get_chunk_size(&self) -> usize {
    msg::DATA::SIZE.min(self.buffer.len())
  }

  /// sends a block of data ... the first available chunk in buffer
  async fn send_data(&mut self) -> Option<bool> {
    let body_len = self.get_chunk_size();
    if let Some(data) = msg::DATA::new(self.seqno, &self.buffer[..body_len]) {
      let mesg = data.serialize();                                
      if let Err(_e) = self.sock.send_to(&mesg, self.server).await {
        self.estado = Estado::Finish;
        self.status = Status::Unknown;        
      } else {
        return Some(body_len < msg::DATA::SIZE);
      }
    }
    None
  }    
  
  /// retransmits last block of data
  /// if max retransmissions are exceeded, finishes the FSM
  async fn retransmit(&mut self) {
    if self.retries < self.max_retries {
      self.retries+=1;
      self.send_data().await;
    } else {
      self.estado = Estado::Finish;
      self.status = Status::MaxRetriesExceeded;
    }
  }

  /// sends next block of data, and updates state accordingly
  async fn send_next(&mut self) {
    match self.send_data().await {
      Some(true) => self.estado = Estado::FinishTX,
      Some(false) => self.estado = Estado::TX,
      None => self.estado = Estado::Finish
    }
  }

  /// FSM handler for state InitTX
  async fn handle_init_tx(&mut self, ev: Evento) {
    println!("init-tx");
    match ev {
      Evento::Timeout => {
        // aborts
        self.estado = Estado::Finish;
        self.status = Status::Timeout;
      }
      Evento::Msg(buffer) => {
          if let Some(mesg) = msg::from_bytes(buffer) {
              match mesg {
                  msg::Mensagem::Ack(ack) => {
                      if ack.block == 0 {
                          self.seqno = 1;
                          self.retries = 0;
                          self.send_next().await;
                                                    
                      } else {
                        self.estado = Estado::Finish;
                      }                   
                  }
                  msg::Mensagem::Err(err) => {
                      self.estado = Estado::Finish;
                      self.status = Status::Error(err.err_code)
                  }
                  _ => {

                  }
              }
          }
      }
      _ => {
        println!("Alguma outra coisa ...");
      }
  }    
}

  /// FSM handler for state FinishTx
  async fn handle_finish_tx(&mut self, ev: Evento) {
    println!("finish-tx");
    match ev {
      Evento::Timeout => {
        self.retransmit().await;
      }
      Evento::Msg(buffer) => {
          if let Some(mesg) = msg::from_bytes(buffer) {
              match mesg {
                  msg::Mensagem::Ack(ack) => {
                      if ack.block == self.seqno {
                        self.estado = Estado::Finish;
                      }                    
                  }
                  msg::Mensagem::Err(err) => {
                      self.estado = Estado::Finish;
                      self.status = Status::Error(err.err_code)
                  }
                  _ => {

                  }
              }
          }
      }
      _ => {
        println!("Alguma outra coisa ...");
      }
    }
  }

  /// FSM handler for state TX
  async fn handle_tx(&mut self, ev: Evento) {
    println!("tx");
    match ev {
      Evento::Timeout => {
        // retransmits a block
        self.retransmit().await;
      }
      Evento::Msg(buffer) => {
          if let Some(mesg) = msg::from_bytes(buffer) {
              match mesg {
                  msg::Mensagem::Ack(ack) => {
                      if ack.block == self.seqno {
                          self.seqno += 1;
                          self.retries = 0;
                          let _chunk = self.buffer.split_to(self.get_chunk_size());
                          self.send_next().await;
                      }                    
                  }
                  msg::Mensagem::Err(err) => {
                      self.estado = Estado::Finish;
                      self.status = Status::Error(err.err_code)
                  }
                  _ => {

                  }
              }
          }
      }
      _ => {
        println!("Alguma outra coisa ...");
      }
  }    
}
}

#[derive(Debug)]
pub struct ClienteTFTP {
    server: String,
    port: u16
}

impl ClienteTFTP {
    pub fn new(server: &str, port: u16) -> Self {
        ClienteTFTP {
            server: server.to_owned(),
            port: port
        }
    }

    async fn do_send(&self, fname: &str, data: &[u8]) -> Option<Sessao> {
        if let Some(mut sessao) = Sessao::new(&self.server, self.port, 1, 3).await {
          sessao.send(fname, &data).await;                                

          return Some(sessao);
        }
        None
      }
      
      async fn do_receive(&self, fname: &str) -> Option<Sessao> {
        if let Some(mut sessao) = Sessao::new(&self.server, self.port, 1, 3).await {
          sessao.receive(fname).await;

          return Some(sessao);
        }
        None
      }

    pub fn envia(&self, fname: &str, rname: &str) -> Status {
        let rt = tokio::runtime::Runtime::new().expect("");
        // let mut rt = tokio::runtime::Builder::new_multi_thread()
        // .worker_threads(1)
        // .enable_all()
        // .build()
        // .expect("Não conseguiu iniciar runtime !");
    
        let data = fs::read(fname).expect("unable to read local file");
        if let Some(sessao) = rt.block_on(self.do_send(rname, &data)) {
          sessao.status
        } else {
          Status::Unknown
        }

    }

    pub fn recebe(&self, fname: &str, local: &str) -> Status {
        let rt = tokio::runtime::Runtime::new().expect("");
        // let mut rt = tokio::runtime::Builder::new_multi_thread()
        // .worker_threads(1)
        // .enable_all()
        // .build()
        // .expect("Não conseguiu iniciar runtime !");
    
        let r = rt.block_on(self.do_receive(fname));
        if let Some(sessao) = r {
          fs::write(local, sessao.buffer).expect("unable to save local file");
          sessao.status
        } else {
          Status::Unknown
        }
    }
}


// async fn talk(server:&str, port: u16) -> io::Result<()> {
//     let sock = UdpSocket::bind("0.0.0.0:0").await?;
//     let msg:Vec<u8> = vec![0xaa, 0xbb, 0xcc, 0xdd, 0xee];
//     let addr = format!("{}:{}", server, port);

//     for k in 0..10 {
//       let _n = sock.send_to(&msg, &addr).await?;
//       let mut buffer = vec![0u8; 1024];
//       let (_rx, _peer) = sock.recv_from(&mut buffer).await?;
//     }

//     Ok(())
// }
