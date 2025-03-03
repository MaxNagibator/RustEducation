use postgres::{Client, NoTls, Error};

fn main() -> Result<(), Error>{
    println!("1");

    let mut client = Client::connect(
        "postgresql://postgres:RjirfLeyz@localhost:5432/money-dev2",
        NoTls,
    )?;

    println!("2");

    for row in client.query("SELECT id, name FROM public.debt_owners", &[])? {
        let id: i32 = row.get(0);
        let username: &str = row.get(1);
        println!(
            "found app user: {}) {}",
            id, username
        );

        println!("3");
    }

    Ok(())
}
