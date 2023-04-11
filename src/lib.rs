use std::collections::HashMap;
#[feature(rt)]
use std::time::Duration;
//use std::fmt;
use tokio::{runtime, sync::oneshot};
use tokio::sync::oneshot::{Receiver,Sender};
use tokio::net::UdpSocket;
use std::fs;
use bytes::BytesMut;
use std::io;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
mod msg;

use msg::Codec;

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
  estado: Estado
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

impl Sessao {
  async fn new(server:&str, port:u16, timeout: u16, retries: u16) -> Self {
    let ip:Vec<u8> = server.split('.')
                           .map(|c| c.parse::<u8>().expect("IP inválido"))
                           .collect();
    if ip.len() != 4 {
      panic!("ip inválido");
    }                          
    let ip = Ipv4Addr::new(ip[0], ip[1], ip[2], ip[3]);
    Sessao {
      sock: UdpSocket::bind("0.0.0.0:0").await.expect("ao criar socket UDP"),
      server: SocketAddr::new(IpAddr::V4(ip), port),
      tid: false,
      buffer: BytesMut::new(),
      seqno: 1,
      timeout: timeout,
      retries: 0,
      max_retries: retries,
      estado: Estado::Idle
    }
  }

  fn is_finished(&self) -> bool {
    self.estado == Estado::Finish
  }

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

  async fn send(&mut self, fname: &str, data: &[u8]) {
    if self.estado != Estado::Idle {
        panic!("sessão em uso");
    }
    
    if let Some(req) = msg::Requisicao::new_wrq(fname, msg::Modo::Octet) {
      let mesg = req.serialize();
      println!("wrq: {:?}", mesg);
      let n = self.sock.send_to(&mesg, self.server).await;
      self.buffer.extend_from_slice(data);
      self.estado = Estado::InitTX;
      self.run().await;
    }
  }

  async fn receive(&mut self, fname: &str) -> Option<()>{
    if self.estado != Estado::Idle {
        panic!("sessão em uso");
    }
    if let Some(req) = msg::Requisicao::new_rrq(fname, msg::Modo::Octet) {
        let mesg = req.serialize();
        println!("rrq: {:?}", mesg);
        let n = self.sock.send_to(&mesg, self.server).await;
        self.estado = Estado::RX;
        self.run().await;
        Some(())
    } else {
        None
    }
    
  }

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
  
  async fn handle_rx(&mut self, ev: Evento) {
    println!("rx");

    match ev {
        Evento::Timeout => {
            self.estado = Estado::Finish;
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
                            self.sock.send_to(&mesg, self.server).await;
                        }                        
                    }
                    msg::Mensagem::Err(err) => {
                        self.estado = Estado::Finish;

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

  fn get_chunk_size(&self) -> usize {
    msg::DATA::SIZE.min(self.buffer.len())
  }

  async fn send_data(&self) -> Option<bool> {
    let body_len = self.get_chunk_size();
    if let Some(data) = msg::DATA::new(self.seqno, &self.buffer[..body_len]) {
      let mesg = data.serialize();                                
      self.sock.send_to(&mesg, self.server).await;
      return Some(body_len < msg::DATA::SIZE);
    }
    None
  }    
  
  async fn retransmit(&mut self) {
    if self.retries < self.max_retries {
      self.retries+=1;
      self.send_data().await;
    } else {
      self.estado = Estado::Finish;
    }
  }

  async fn send_next(&mut self) {
    match self.send_data().await {
      Some(true) => self.estado = Estado::FinishTX,
      Some(false) => self.estado = Estado::TX,
      None => self.estado = Estado::Finish
    }
  }

  async fn handle_init_tx(&mut self, ev: Evento) {
    println!("init-tx");
    match ev {
      Evento::Timeout => {
        // aborts
        self.estado = Estado::Finish;
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
                          self.buffer.split_to(self.get_chunk_size());
                          self.send_next().await;
                      }                    
                  }
                  msg::Mensagem::Err(err) => {
                      self.estado = Estado::Finish;
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

    async fn do_send(&self, fname: &str) -> Option<Sessao> {
        if let Ok(data) = fs::read(fname) {
            
            let mut sessao = Sessao::new(&self.server, self.port, 1, 3).await;
            sessao.send(fname, &data).await;                                

            Some(sessao)
        } else {
            None
        }
      }
      
      async fn do_receive(&self, fname: &str) -> Option<Sessao> {
        let mut sessao = Sessao::new(&self.server, self.port, 1, 3).await;
        sessao.receive(fname).await;

        Some(sessao)
      }

    pub fn envia(&self, fname: &str) {
        let mut rt = tokio::runtime::Runtime::new().expect("");
        // let mut rt = tokio::runtime::Builder::new_multi_thread()
        // .worker_threads(1)
        // .enable_all()
        // .build()
        // .expect("Não conseguiu iniciar runtime !");
    
        let r = rt.block_on(self.do_send(fname));    
    }

    pub fn recebe(&self, fname: &str, local: &str) -> io::Result<()>{
        let mut rt = tokio::runtime::Runtime::new().expect("");
        // let mut rt = tokio::runtime::Builder::new_multi_thread()
        // .worker_threads(1)
        // .enable_all()
        // .build()
        // .expect("Não conseguiu iniciar runtime !");
    
        let r = rt.block_on(self.do_receive(fname));
        if let Some(sessao) = r {
          fs::write("local", sessao.buffer)?;
        }
        Ok(())
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
