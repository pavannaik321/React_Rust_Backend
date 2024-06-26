use postgres::{Client,NoTls};
use postgres::Error as PostgresError;
use std::net::{TcpListener , TcpStream};
use std::io::{Read,Write};
use std::env;

#[macro_use]
extern crate serde_derive;

// Model : Product struct with id , name , price
#[derive(Serialize,Deserialize)]
struct  Product {
    id : Option<i32>,
    name : String,
    price : i32
}

// Database url
const DB_URL : &str = env!("DATABASE_URL");

// Constants
const OK_RESPONSE: &str = "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n";
const NOT_FOUND: &str = "HTTP/1.1 404 NOT FOUND\r\n\r\n";
const INTERNAL_SERVER_ERROR: &str = "HTTP/1.1 500 INTERNAL SERVER ERROR\r\n\r\n";

// Main function
fn main(){
    //set database
    if let  Err(e) = set_database() {
        println!("Error : {}",e);
        return ;
    }

    // start server and print port
    let listener = TcpListener::bind(format!("0.0.0.0:8080")).unwrap();
    println!("Server started at port 8080");

    //handle the client
    for stream in listener.incoming(){
        match stream{
            Ok(stream)=>{
                handle_client(stream);
            }
            Err(e)=>{
                println!("Error : {}",e);
            }
        }
    }
}

// handle_client function(to handle all the routes)
fn handle_client(mut stream:TcpStream){
    let mut buffer = [0; 1024]; //we want the input to be not more than this range
    let mut request = String::new();

    //create routes
    match stream.read(&mut buffer){
        Ok(size)=>{
            request.push_str(String::from_utf8_lossy(&buffer[..size]).as_ref());

            let (status_line,content) = match &*request{
                r if r.starts_with("POST /products") => handle_post_request(r),
                r if r.starts_with("GET /products/") => handle_get_request(r),
                r if r.starts_with("GET /products") => handle_get_all_request(r),
                r if r.starts_with("GET /price/")=> handle_price_get_request(r),
                r if r.starts_with("PUT /products/") => handle_put_request(r),
                r if r.starts_with("DELETE /products/") => handle_delete_request(r),
                _=>(NOT_FOUND.to_string()," 404 Not found".to_string()), //if no routes match

            };
            
            //send response back to user handle the response
            stream.write_all(format!("{}{}",status_line,content).as_bytes()).unwrap();
        }
        Err(e)=>{
            println!("Error: {}",e);
        }
    }

}

// CONTROLLERS

// handle_post_request function
fn handle_post_request(request: &str)->(String,String){
    match (get_product_request_body(&request),Client::connect(DB_URL,NoTls)){
        (Ok(product),Ok(mut client))=>{
            client.execute(
                "INSERT INTO products (name,price) VALUES ($1,$2)",
                &[&product.name,&product.price]
            ).unwrap();
            (OK_RESPONSE.to_string(),"Product created".to_string())
        }
        _=>(INTERNAL_SERVER_ERROR.to_string(), "Error".to_string()),
    }
}

// handle_get_request function
fn handle_get_request(request: &str) ->(String,String){
    match(get_id(&request).parse::<i32>() , Client::connect(DB_URL,NoTls)){
        (Ok(id),Ok(mut client))=>
        match client.query_one("SELECT * FROM  products WHERE id=$1",&[&id]) { // like if statement
            Ok(row) => {
                let product = Product{
                    id: row.get(0),
                    name: row.get(1),
                    price: row.get(2),
                };
                (OK_RESPONSE.to_string(), serde_json::to_string(&product).unwrap())
            }
            _=>(NOT_FOUND.to_string(),"Product not found".to_string()),
        }
        _=>(INTERNAL_SERVER_ERROR.to_string(),"Error".to_string()),
    }
}

// fetch below the price range
fn handle_price_get_request(request: &str) -> (String, String) {
    match (get_id(&request).parse::<i32>(), Client::connect(DB_URL, NoTls)) {
        (Ok(price), Ok(mut client)) => {
            let mut prices = Vec::new();
            for row in client.query("SELECT * FROM products WHERE price<$1", &[&price]).unwrap() {
                prices.push(Product{
                    id: row.get(0),
                    name: row.get(1),
                    price: row.get(2),
                });
            }
            (OK_RESPONSE.to_string(), serde_json::to_string(&prices).unwrap())
        }
        _ => (INTERNAL_SERVER_ERROR.to_string(), "Error".to_string()),
    }
}


//handle_get_all_request function
fn handle_get_all_request(request: &str)->(String ,String){
    match Client::connect(DB_URL,NoTls){
        Ok(mut client)=>{
            let mut products = Vec::new();
 
            for row in client.query("SELECT * FROM products",&[]).unwrap(){
                products.push(Product {
                    id: row.get(0),
                    name: row.get(1),
                    price: row.get(2),
                });
            }
            (OK_RESPONSE.to_string(), serde_json::to_string(&products).unwrap())
        }
        _=> (INTERNAL_SERVER_ERROR.to_string(),"Error".to_string()),
    }
}


//handl_put_request function
fn handle_put_request(request: &str)->(String , String){
    match(
        get_id(&request).parse::<i32>(),
        get_product_request_body(&request),
        Client::connect(DB_URL,NoTls),
    ){
        (Ok(id),Ok(product),Ok(mut client))=>{
            client.execute("UPDATE products SET name = $1 , email = $2 WHERE id = $3",&[&product.name, &product.price, &id])
            .unwrap();
            
            (OK_RESPONSE.to_string(),"Product updated".to_string())
        }
        _=> (INTERNAL_SERVER_ERROR.to_string(),"Error".to_string()),
    }
}


// handle_delete_request function
fn handle_delete_request(request: &str)->(String , String){
    match (get_id(&request).parse::<i32>(),Client::connect(DB_URL,NoTls)){
        (Ok(id),Ok(mut client))=>{
           let rows_affected = client.execute("DELETE FROM products WHERE id=$1",&[&id]).unwrap();

           if rows_affected==0 {
            return (NOT_FOUND.to_string(),"User not found".to_string());
           }

            (OK_RESPONSE.to_string(),"Product deleted".to_string())
        }
        _=> (INTERNAL_SERVER_ERROR.to_string(),"Error".to_string()),
    }
}



// set database function
fn set_database()->Result<(),PostgresError>{
    //connect to database
    let mut client = Client::connect(DB_URL, NoTls)?;


    // Create table
    client.batch_execute(
        "CREATE TABLE IF NOT EXISTS products (
            id SERIAL PRIMARY KEY,
            name  VARCHAR NOT NULL,
            price  INT NOT NULL
         )"
    )?;
    Ok(())
}

// get_id function
fn get_id(request: &str)->&str{
    request.split("/").nth(2).unwrap_or_default().split_whitespace().next().unwrap_or_default()
}

// fn get_price(request: &str)->&str{
//     println!("{}",request);
//     request.split("/").nth(5).unwrap_or_default().split_whitespace().next().unwrap_or_default()
// }
//deserialize product from request body with the id
fn get_product_request_body(request: &str)->Result<Product,serde_json::Error>{
    serde_json::from_str(request.split("\r\n\r\n").last().unwrap_or_default())
}