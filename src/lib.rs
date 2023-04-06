use std::collections::HashMap;
#[feature(rt)]
use std::time::Duration;
//use std::fmt;
use tokio::{runtime, sync::oneshot};
use tokio::sync::oneshot::{Receiver,Sender};
use tokio::net::UdpSocket;
use std::fs;
use bytes::BytesMut;
use std::net::SocketAddr;
mod msg;

use msg::Codec;

#[derive(Debug)]
struct Sessao {
  sock: UdpSocket,
  server: String,
  port: u16,
  buffer: BytesMut,
  seqno: u16,
  finished: bool,
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
  TX,
  Finish
}


impl Sessao {
  async fn new(server:&str, port:u16, timeout: u16, retries: u16) -> Self {
    Sessao {
      sock: UdpSocket::bind("0.0.0.0:0").await.expect("ao criar socket UDP"),
      server: server.to_owned(),
      port: port,
      buffer: BytesMut::new(),
      seqno: 1,
      finished: false,
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
          Estado::TX => {
            self.handle_tx(ev).await;
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
    self.buffer.extend_from_slice(data);
    self.estado = Estado::TX;
    

  }

  fn get_server_addr(&self) -> String {
    format!("{}:{}", self.server, self.port)
  }

  async fn receive(&mut self, fname: &str) -> Option<()>{
    if self.estado != Estado::Idle {
        panic!("sessão em uso");
    }
    if let Some(req) = msg::Requisicao::new_rrq(fname, msg::Modo::Octet) {
        let mesg = req.serialize();
        let n = self.sock.send_to(&mesg, self.get_server_addr()).await;
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
          let msg = buf[..len].to_vec();
          self.port = addr.port();
          return Evento::Msg(msg);
        }
      }
    }
    Evento::Nada
  }
  
//   async fn run_event(&mut self) {
//     let ev = self.get_event().await;
//     match self.estado {
//       Estado::RX => {
//         self.handle_rx(ev).await;
//       }
//       Estado::TX => {
//         self.handle_tx(ev).await;
//       }
//       Estado::Finish => {
//       }
//     }   

//   }

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
                            self.sock.send_to(&mesg, self.get_server_addr()).await;
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

  async fn handle_tx(&mut self, ev: Evento) {
      println!("tx");
      match ev {
          Evento::Timeout => {
            self.seqno += 1;
            if self.seqno == self.retries {
              self.estado = Estado::Finish;
            }
            println!("Timeout {}", self.seqno);                
          }
          Evento::Msg(msg) => {
            println!("Adicionando {} bytes ao buffer", msg.len());
            self.buffer.extend_from_slice(&msg);
            self.estado = Estado::RX;
            self.seqno = 1;
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

    pub fn recebe(&self, fname: &str) {
        let mut rt = tokio::runtime::Runtime::new().expect("");
        // let mut rt = tokio::runtime::Builder::new_multi_thread()
        // .worker_threads(1)
        // .enable_all()
        // .build()
        // .expect("Não conseguiu iniciar runtime !");
    
        let r = rt.block_on(self.do_receive(fname));    
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
