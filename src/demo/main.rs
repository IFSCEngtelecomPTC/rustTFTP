// Include the `items` module, which is generated from items.proto.
// It is important to maintain the same structure as in the proto.
pub mod tftp2 {
    pub mod spec {
        include!(concat!(env!("OUT_DIR"), "/tftp2.rs"));
    }
  }
  
use tftp2::spec;
use prost;
use bytes::BytesMut;
use prost::Message;

fn main() {
    let mut msg = spec::Req::default();
    msg.set_mode(spec::Mode::Octet);
    msg.fname = String::from("teste.txt");
    println!("{:?}", msg);

    let mut buffer = BytesMut::new();
    let data = msg.encode(&mut buffer);
    println!("encoded={:?}", buffer);
    let msg = spec::Req::decode(buffer);    
    
    // let msg:Result<spec::Req, prost::DecodeError> = prost::Message::decode(buffer);    
    match msg {
        Ok(msg) => {
            println!("decoded: {:?}", msg);
        }
        Err(e) => {
            println!("Erro: {:?}", e);
        }
    }
}
