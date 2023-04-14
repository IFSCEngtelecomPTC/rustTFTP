// Include the `items` module, which is generated from items.proto.
// It is important to maintain the same structure as in the proto.
pub mod tftp2 {
    pub mod spec {
        include!(concat!(env!("OUT_DIR"), "/tftp2.rs"));
    }
  }
  
use tftp2::spec::{mensagem::Msg, *};
use prost;
use bytes::BytesMut;
use prost::Message;

fn main() {
    let mut msg = Mensagem::default();
    let mut inner = Req::default();
    // msg.msg = Some(spec::(inner));
    println!("{:?}", msg);
   
    inner.set_mode(Mode::Octet);
    inner.fname = String::from("teste.txt");
    msg.msg = Some(mensagem::Msg::Wrq(inner));

    let mut buffer = BytesMut::new();
    let data = msg.encode(&mut buffer);
    println!("encoded={:?}", buffer);
    let msg = Mensagem::decode(buffer);    
    
    match msg {
        Ok(msg) => {            
            println!("decoded: {:?}", msg);
            if let Some(msg) = msg.msg {
                match msg {
                    Msg::Rrq(inner) => {
                        println!("rrq: {:?}", inner);
                    }
                    Msg::Wrq(inner) => {
                        println!("wrq: {:?}", inner);
                    }
                    Msg::Data(inner) => {
                        println!("data: {:?}", inner);
                    }
                    Msg::Ack(inner) => {
                        println!("ack: {:?}", inner);
                    }
                    Msg::Error(inner) => {
                        println!("err: {:?}", inner);
                    }
                    _ => {
                        println!("Alguma outra coisa ...");
                    }
                }    
            }
        }
        Err(e) => {
            println!("Erro: {:?}", e);
        }
    }
    
}
