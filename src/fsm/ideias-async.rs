use std::collections::HashMap;
#[feature(rt)]
use std::time::Duration;
//use std::fmt;
use tokio::{runtime, sync::oneshot};
use tokio::sync::oneshot::{Receiver,Sender};
use tokio::net::UdpSocket;

#[derive(Debug)]
struct Protocolo {
  sock: UdpSocket,
  buffer: Vec<u8>,
  seqno: u16,
  finished: bool,
  timeout: u16,
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

/// Pode-se criar uma função para receber eventos, e retorná-los encapsulados em um enum !
async fn wait_event(proto: &Protocolo) -> Evento {
  let mut buf = [0; 1024];
  tokio::select! {
    val = tokio::time::sleep(Duration::from_secs(proto.timeout as u64)) => {
      println!("Timeout: val={:?}", val);
      return Evento::Timeout;
    }
    val = proto.sock.recv_from(&mut buf) => {
      if let Ok((len,addr)) = val {
        println!("Rx: val={:?}", (len,addr));
        let msg = buf[..len].to_vec();
        return Evento::Msg(msg);
      }
    }
  }
  Evento::Nada
}

#[derive(Debug)]
#[derive(PartialEq)]
enum Estado {
  RX,
  TX,
  Finish
}


impl Protocolo {
  async fn new(addr: &str, timeout: u16) -> Self {
    Protocolo {
      sock: UdpSocket::bind(addr).await.expect("ao criar socket UDP"),
      buffer: vec![],
      seqno: 1,
      finished: false,
      timeout: timeout,
      estado: Estado::RX
    }
  }

  fn is_finished(&self) -> bool {
    self.estado == Estado::Finish
  }

  async fn get_event(&self) -> Evento {
    let mut buf = [0; 1024];

    tokio::select! {
      val = tokio::time::sleep(Duration::from_secs(self.timeout as u64)) => {
        println!("Timeout: val={:?}", val);
        return Evento::Timeout;
      }
      val = self.sock.recv_from(&mut buf) => {
        if let Ok((len,addr)) = val {
          println!("Rx: val={:?}", (len,addr));
          let msg = buf[..len].to_vec();
          return Evento::Msg(msg);
        }
      }
    }
    Evento::Nada
  }
  
  async fn run_event(&mut self) {
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
    }   

  }

  async fn handle_rx(&mut self, ev: Evento) {
    println!("rx");

    match ev {
        Evento::Timeout => {
          self.seqno += 1;
          println!("Timeout {}", self.seqno);        
        }
        Evento::Msg(msg) => {
          println!("Adicionando {} bytes ao buffer", msg.len());
          self.buffer.extend_from_slice(&msg);
          // tokio::spawn(async {handle_tx(proto, chan).await;});
          self.estado = Estado::TX;
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
            if self.seqno == 10 {
              self.estado = Estado::Finish;
            }
            println!("Timeout {}", self.seqno);                
          }
          Evento::Msg(msg) => {
            println!("Adicionando {} bytes ao buffer", msg.len());
            self.buffer.extend_from_slice(&msg);
            self.estado = Estado::RX;
          }
          _ => {
            println!("Alguma outra coisa ...");
          }
      }
  }        
}


async fn run_proto() -> Option<Protocolo> {
  let mut proto = Protocolo::new("0.0.0.0:1111", 2).await;
  // let (tx, mut rx) = oneshot::channel();

  while ! proto.is_finished() {
    proto.run_event().await;
  }
  Some(proto)
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

fn main() {
    let mut rt = tokio::runtime::Runtime::new().expect("");
    // let mut rt = tokio::runtime::Builder::new_multi_thread()
    // .worker_threads(1)
    // .enable_all()
    // .build()
    // .expect("Não conseguiu iniciar runtime !");

    println!("Started task!");
    let r = rt.block_on(run_proto());
    println!("Stopped task: {:?}", r);
}
