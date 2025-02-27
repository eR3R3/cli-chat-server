use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::thread;

//when there is a new client trying to connect, the accept function will receive one time,
//and handle the rest of the thing in the separate thread we create

const LOCAL: &str = "127.0.0.1";
const MSG_SIZE: usize = 100;

fn main() {
    //create a server
    let server = TcpListener::bind(LOCAL).expect("Listener failed to bind");
    //set it to non-blocking(cannot use .incoming in non-blocking)
    server.set_nonblocking(true).expect("Failed to set nonblocking");

    //create a clients that store all the data
    let mut clients: Arc<Mutex<Vec<_>>> = Arc::new(Mutex::new(Vec::new()));
    //create channel to send data between the threads
    let (tx, rx) = std::sync::mpsc::channel::<String>();

    //start to read from the client sides, loop and wait for connection
    loop {
        //fetch the stream(which is like a two-sided channel) from the client side
        //make stream mut because it needs to be read
        if let Ok((mut stream, addr)) = server.accept() {
            //add the stream into the clients and store it
            clients.lock().unwrap().push(stream.try_clone().expect("Failed to clone client"));
            //clone the sender so that I can move the ownership into the thread
            let tx = tx.clone();
            thread::spawn(move || {
                loop {
                    //create a buffer to read the data
                    let mut buff: Vec<u8> = vec![0; MSG_SIZE];
                    //read the data from the mut stream
                    match stream.read_exact(&mut buff).expect("Failed to read from client") {
                        Err(0) => break,
                        Ok(_) => {
                            let msg = buff.into_iter().take_while(|&x| x != 0).collect::<Vec<_>>();
                            //make the data Cow<str>
                            let msg = String::from_utf8_lossy(&msg);
                            //make the data &str
                            let msg = msg.trim();
                            //send the data to other thread
                            tx.send(msg.to_string()).expect("Failed to send message");
                        }
                        Err(e) if e.kind() == &std::io::ErrorKind::WouldBlock => {
                            //skip it if it
                            thread::sleep(std::time::Duration::from_millis(100));
                            continue;
                        }
                        Err(_) => {
                            print!("client disconnected");
                            break
                        }
                    }
                }
            })
        }

        if let Ok(msg) = rx.try_recv() {
            clients.lock().unwrap().iter_mut().for_each(|stream| {
                stream.write_all(&msg.as_bytes()).expect("Failed to write to client");
            })
        }
    }
}
