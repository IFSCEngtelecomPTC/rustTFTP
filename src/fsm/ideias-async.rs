use async_std::{fs::File, io, prelude::*, 
                task, future,
                net::UdpSocket,
                channel::*};
use std::time::Duration;
//use std::fmt;

#[derive(Debug)]
struct Protocolo {
  buffer: Vec<u8>,
  seqno: u16
}

/// uma transicao eh a criacao de uma task com o respectivo 
/// tratador do estado. Isso vale inclusive para auto-transicao (poderia
/// simplificar ?). A instancia do protocolo eh passada e consumida por cada
/// tratador ... cada um passa essa instancia para o tratador seguinte.
/// A MEF fica bem diferente ... ela inicia com o primeiro tratador, que espera por eventos. Ao recebê-los,
/// processa-os e então executa as respectivas transições (cria nova task para o próximo tratador).
/// Quando a MEF chega a um estado terminal, ela envia a instância de Protocolo pelo canal. 
async fn handle_rx(mut proto: Protocolo, chan: Sender<Protocolo>) -> io::Result<Option<Protocolo>> {
  println!("rx");
  proto.seqno += 1;
  task::spawn(async {handle_tx(proto, chan).await;});
  Ok(None)
}

async fn handle_tx(mut proto: Protocolo, chan: Sender<Protocolo>) -> io::Result<Option<Protocolo>> {
  println!("tx");
  chan.send(proto).await;
  Ok(None)  
}

/*
async fn handle_init_tx(proto: &mut Protocolo) -> io::Result<()> {
  task::spawn(async {handle_finish(proto).await;});
  Ok(())  
}

async fn handle_finish(proto: &mut Protocolo) -> io::Result<()> {
  Ok(())    
}
*/

async fn run_proto() -> io::Result<Option<Protocolo>> {
  let proto = Protocolo {
    buffer: vec![1,2,3,4,5],
    seqno: 5
  };
  let (s,r) = bounded::<Protocolo>(1);
  task::spawn(async {handle_rx(proto, s).await;});  
  if r.is_closed() {
    println!("Ops ... canal fechado !");
  }
  let result = r.recv().await;
  println!("result: {:?}", result);
  if let Ok(proto) = result {
    println!("proto ok");
    return Ok(Some(proto));
  } else {
    println!("proto err");
    return Ok(None);
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

// async fn spiral(dt: u32) -> io::Result<()> {
//     let mut stdout = io::stdout();
//     stdout.write_all(b"Timer start !\n").await?;
//     for k in 0..10 {
//       task::sleep(Duration::from_secs(dt as u64)).await;
//       stdout.write_all(format!("Timer: {}\n", k).as_bytes()).await?;
//     }
//     Ok(())
// }

fn main() {
    // let reader_task = task::spawn(async {
    //     let client = talk("191.36.13.62", 53);
    //     let result = future::timeout(Duration::from_secs(5), client).await;
    //     match result {
    //         Ok(_) => {},
    //         Err(e) => println!("Error talking to server: {:?}", e),
    //     }
    // });
    // let tout_task = task::spawn(async {
    //     let result = spiral(1).await;
    //     match result {
    //         Err(e) => println!("Error in timer: {:?}", e),
    //         _ => {}
    //     }
    // });
    
    println!("Started task!");
    //task::block_on(reader_task);
    let r = task::block_on(task::spawn(run_proto()));
    println!("Stopped task: {:?}", r);
}