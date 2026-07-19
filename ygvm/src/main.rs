extern crate core;

use crate::syntax::lexer::Lexer;
use crate::syntax::parser::Parser;
use crate::syntax::pretty::pretty_print;
use crate::utils::socket::client::Client;
use crate::utils::socket::server::Server;
use crate::vm::module::VMModuleManager;
use crate::vm::{VMRef, VM};
use ::std::thread;
use ::std::time::Duration;

pub mod syntax;
pub mod vm;
pub mod std;
pub mod napi;
pub mod utils;

fn main() {
    // check_lexer();
    // check_parser();
    check_vm();
    // check_socket()
}

#[allow(unused)]
pub fn check_socket() {
    thread::spawn(|| {
        let server = Server::new("127.0.0.1:25565".to_owned()).unwrap();
        loop {
            if let Some((mut connection, addr)) = server.accept().unwrap() {
                println!("[S] New connection: {:?}", addr);
                connection.send("Hello, World!".to_owned()).unwrap();
                loop {
                    if let Some(message) = connection.recv().unwrap() {
                        thread::sleep(Duration::from_millis(1000));
                        println!("[S] Recv: {}", message);
                        connection.send("Hello, World!".to_owned()).unwrap();
                    } else {
                        thread::sleep(Duration::from_millis(100));
                    }
                }
            } else {
                thread::sleep(Duration::from_millis(100));
            }
        }
    });

    thread::spawn(|| {
        let mut counter = 0;
        let mut connection = Client::new("127.0.0.1:25565".to_owned()).unwrap();
        println!("[C] Connection: {:?}", connection);
        loop {
            if let Some(message) = connection.recv().unwrap() {
                thread::sleep(Duration::from_millis(1000));
                println!("[C] Recv: {}", message);
                connection.send("Hello, User!".to_owned()).unwrap();
                counter += 1;
                if counter >= 2 {
                    println!("[C] Close!");
                    connection.close().unwrap();
                    return;
                }
            } else {
                thread::sleep(Duration::from_millis(100));
            }
        }
    });

    loop {}
}

#[allow(unused)]
pub fn check_vm() {
    let mut vm = VM::new().unwrap();
    let mut vm = VMRef::from(&mut vm);

    let input = ::std::fs::read_to_string("examples/chat.yg").unwrap();
    let lexer = Lexer::new("test.yg".to_owned(), input);
    let mut parser = Parser::new(lexer);
    let module = parser.parse_module().unwrap();
    VMModuleManager::load_ast_module(vm, &module).unwrap();

    vm.stop(false).unwrap();
}

#[allow(unused)]
fn check_parser() {
    println!("[Parser]\n");
    let input = ::std::fs::read_to_string("examples/chat.yg").unwrap();
    let lexer = Lexer::new("full.yg".to_owned(), input);
    let mut parser = Parser::new(lexer);
    let module = parser.parse_module().unwrap();
    println!("{}", pretty_print(&module));
    println!();
}

#[allow(unused)]
fn check_lexer() {
    println!("[Lexer]\n");
    let input = ::std::fs::read_to_string("examples/chat.yg").unwrap();
    let mut lexer = Lexer::new("full.yg".to_owned(), input);
    while let Some(token) = lexer.next().unwrap() {
        println!("[{}, {}] {:?}", token.debug.line, token.debug.column, token.data)
    }
    println!();
}

pub(crate) fn ownership_hack_mut<'a, 'b, T>(value: &'a mut T) -> &'b mut T {
    // SAFETY: Обход владения.
    unsafe { ::std::mem::transmute(value) }
}

pub(crate) fn ownership_hack<'a, 'b, T>(value: &'a T) -> &'b T {
    // SAFETY: Обход владения.
    unsafe { ::std::mem::transmute(value) }
}