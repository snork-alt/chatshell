fn main() {
    println!("Hello, World!");
    println!("Welcome to your basic Rust program!");
    
    // Demonstrate some basic Rust features
    let name = "Rust";
    let version = 1.0;
    
    println!("This program is written in {} version {}", name, version);
    
    // A simple function call
    greet_user("Developer");
}

fn greet_user(user: &str) {
    println!("Hello, {}! Thanks for running this Rust program.", user);
} 