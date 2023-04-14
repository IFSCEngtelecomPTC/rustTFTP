use std::time::Duration;
use tokio::net::UdpSocket;
use std::fs;
use bytes::BytesMut;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::str::FromStr;

use tftp2::spec::{mensagem::Msg, *};
use prost::Message;

// Include the `items` module, which is generated from items.proto.
// It is important to maintain the same structure as in the proto.
pub mod tftp2 {
  pub mod spec {
      include!(concat!(env!("OUT_DIR"), "/tftp2.rs"));
  }
}

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
  seqno: u32,
  timeout: u16,
  retries: u16,
  max_retries: u16,
  estado: Estado,
  status: Status
}

#[derive(Debug)]
enum Evento {
  Msg(bytes::BytesMut),
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

pub trait MessageGen {
  fn new_ack(block: u32) -> Self;

  fn new_data(block: u32, body: &[u8]) -> Self;

  fn new_rrq(fname: &str) -> Self;

  fn new_wrq(fname: &str) -> Self;

  fn new_err(code: u32) -> Self;

  fn serialize(self) -> bytes::BytesMut;
}

impl MessageGen for Mensagem {

  // utility function to encode a message
  fn serialize(self) -> bytes::BytesMut {
    let mut buffer = bytes::BytesMut::new();
    self.encode(&mut buffer);                                
    buffer
  }


  /// utility function: generates a Req message. The kind of Req (rrq or wrq)
  /// is specified in the closure f_enc
  fn new_rrq(fname: &str) -> Self {
    let mut msg = Mensagem::default();
    let mut inner = Req::default();
   
    inner.set_mode(Mode::Octet);
    inner.fname = String::from(fname);
    msg.msg = Some(Msg::Rrq(inner));

    return msg;
  }

  fn new_wrq(fname: &str) -> Self {
    let mut msg = Mensagem::default();
    let mut inner = Req::default();
   
    inner.set_mode(Mode::Octet);
    inner.fname = String::from(fname);
    msg.msg = Some(Msg::Wrq(inner));

    return msg;
  }

  fn new_ack(block: u32) -> Self {
    let mut msg = Mensagem::default();
    let mut inner = Ack::default();
   
    inner.block_n = block;
    msg.msg = Some(Msg::Ack(inner));

    return msg;
  }

  fn new_data(block: u32, body: &[u8]) -> Self {
    let mut msg = Mensagem::default();
    let mut inner = Data::default();
   
    inner.block_n = block;
    inner.message.extend(body);
    msg.msg = Some(Msg::Data(inner));

    return msg;
  }


  fn new_err(code: u32) -> Self {
    Mensagem::default()
  }

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
    if let Ok(ip) = Ipv4Addr::from_str(ip) {
      Some(ip)
    } else {
      None
    }
    // let ip:Vec<_> = ip.split('.')
    //                        .map(|c| c.parse::<u8>())
    //                        .collect();
    // if ! ip.iter().any(|x| x.is_err()) {
    //   if ip.len() == 4 {
    //     let ip:Vec<_> = ip.into_iter()
    //                       .map(|x| x.unwrap())
    //                       .collect();
    //     return Some(Ipv4Addr::new(ip[0], ip[1], ip[2], ip[3]));
    //   }
    // }
    // None
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
    
    // creates a Wrq message
    let msg = Mensagem::new_wrq(fname);
    // let msg = Sessao::new_req(fname, |req| mensagem::Msg::Wrq(req));

    // ... and then encodes it
    let mut buffer = msg.serialize();

    // finally, sends the message and starts the FSM
    if let Ok(_n) = self.sock.send_to(&buffer, self.server).await {
      self.buffer.extend_from_slice(data);
      self.estado = Estado::InitTX;
      self.run().await;
    } else {
      self.estado = Estado::Finish;
      self.status = Status::Unknown;
    }
    
  }

  /// starts reception of file "fname".
  /// In the end, its contents are contained in attribute "buffer"
  async fn receive(&mut self, fname: &str) -> Option<()>{
    if self.estado != Estado::Idle {
        panic!("sessão em uso");
    }

    // creates a Rrq message
    let msg = Mensagem::new_rrq(fname);
    // ... and then encodes it
    let mut buffer = msg.serialize();

    // finally, sends the message and starts the FSM
    if let Ok(_n) = self.sock.send_to(&buffer, self.server).await {
      self.estado = Estado::RX;
      self.run().await;
      return Some(());
    }

    self.estado = Estado::Finish;
    self.status = Status::Unknown;
    None        
  }

  /// waits for an event, and return it
  async fn get_event(&mut self) -> Evento {
    let mut buf = bytes::BytesMut::new();
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
            return Evento::Msg(buf);
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
            if let Ok(msg) = Mensagem::decode(buffer) {
                if let Some(msg) = msg.msg {
                  match msg {
                      Msg::Data(data) => {
                          if data.block_n == self.seqno {
                              if self.seqno < 65535 {
                                self.seqno += 1;
                                self.buffer.extend_from_slice(&data.message);
                                if data.message.len() < ClienteTFTP::DATA_SIZE {
                                    self.estado = Estado::Finish;
                                }
                              } else {
                                self.estado = Estado::Finish;
                                self.status = Status::Unknown;
                              }
                          }
                          let resp = Mensagem::new_ack(data.block_n);
                          let mut buffer = resp.serialize();
                          if let Err(_e) = self.sock.send_to(&buffer, self.server).await {
                            self.estado = Estado::Finish;
                            self.status = Status::Unknown;
                          }
                      }
                      Msg::Error(err) => {
                          self.estado = Estado::Finish;
                          self.status = Status::Error(err.errorcode as u16);

                      }
                      _ => {

                      }
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
    ClienteTFTP::DATA_SIZE.min(self.buffer.len())
  }

  /// sends a block of data ... the first available chunk in buffer
  async fn send_data(&mut self) -> Option<bool> {
    // the size of the next block of data
    let body_len = self.get_chunk_size();
    // creates a data message
    let data = Mensagem::new_data(self.seqno as u32, &self.buffer[..body_len]);
    // ... and then encodes it
    let mut buffer = data.serialize();
    // sends the data message, and updates state of the FSM
    if let Err(_e) = self.sock.send_to(&buffer, self.server).await {
      self.estado = Estado::Finish;
      self.status = Status::Unknown;        
    } else {
      // if sent the message, then returns true if it was the last message
      return Some(body_len < ClienteTFTP::DATA_SIZE);
    }

    // no message sent: some error (TODO: i still have to propagates it)
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
        if let Ok(msg) = Mensagem::decode(buffer) {
          if let Some(msg) = msg.msg {
            match msg {
                Msg::Ack(ack) => {
                  if ack.block_n == 0 {
                      self.seqno = 1;
                      self.retries = 0;
                      self.send_next().await;                                                    
                  } else {
                    self.estado = Estado::Finish;
                  } 
                }                  
                Msg::Error(err) => {
                    self.estado = Estado::Finish;
                    self.status = Status::Error(err.errorcode as u16)
                }
                _ => {

                }
              }
          }
        }
      }
      _ => {
        println!("evento desconhecido");
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
        if let Ok(msg) = Mensagem::decode(buffer) {
          if let Some(msg) = msg.msg {
            match msg {
                Msg::Ack(ack) => {
                  if ack.block_n == self.seqno as u32 {
                    self.estado = Estado::Finish;
                  }                    
                }
                Msg::Error(err) => {
                  self.estado = Estado::Finish;
                  self.status = Status::Error(err.errorcode as u16)
                }
                _ => {

                }
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
        if let Ok(msg) = Mensagem::decode(buffer) {
          if let Some(msg) = msg.msg {
            match msg {
                Msg::Ack(ack) => {
                    if ack.block_n == self.seqno {
                      if self.seqno < 65535 {
                        self.seqno += 1;
                        self.retries = 0;
                        let _chunk = self.buffer.split_to(self.get_chunk_size());
                        self.send_next().await;
                      } else {
                        self.estado = Estado::Finish;
                        self.status = Status::Unknown;
                      }
                    }                    
                }
                Msg::Error(err) => {
                    self.estado = Estado::Finish;
                    self.status = Status::Error(err.errorcode as u16)
                }
                _ => {

                }
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
    const DATA_SIZE:usize = 512;

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
