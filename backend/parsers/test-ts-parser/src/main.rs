use windmill_parser_ts::parse_deno_signature;

fn main() -> anyhow::Result<()> {
    let code = r#"

interface Animal {
    name: string
}

    interface User {
  name: string;
  age: number;
  animal: Animal;
}

type User = {
  name: string
}


export async function main(
  user: User,
  person: {name: string; age:number}
) {
  
}

    "#;

    let main_signature = parse_deno_signature(code, false, false, None)?;

//    println!("{:#?}", main_signature);

    Ok(())
}
