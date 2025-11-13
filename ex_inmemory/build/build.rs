use bdmg2k::{doc_generator, schema_generator, sqlite_generator, ObjectDB};

fn main() {
    let result = ObjectDB::load("./objectstore.json");

    println!("cargo:rerun-if-changed=objectstore.json");
    println!("cargo:rerun-if-changed=build.rs");

    if result.is_err() {
        println!(
            "Error while loading the data model: {}",
            result.err().unwrap()
        );
        std::process::exit(1);
    }

    let mut object_db = result.unwrap();

    if let Err(e) = object_db.validate() {
        println!("Error while validating the data model: {}", e);
        std::process::exit(2);
    }

    let output_dir = match std::env::var("OUT_DIR") {
        Ok(value) => value,
        Err(e) => {
            println!("Unable to obtain the value of the environment variable 'OUT_DIR': {e}.");
            std::process::exit(3);
        }
    };

    object_db.set_rust_destination(&output_dir);

    //Write the diesel schema. Great if the database is not meant to be saved and stay in memory
    if let Err(e) = schema_generator::write_schema(&object_db, &output_dir) {
        println!("Error while generating the diesel schema: {e}");
        std::process::exit(4);
    }

    if let Err(e) = sqlite_generator::write_install(&object_db, &output_dir, "up") {
        println!("Error while generating the installation script: {e}");
        std::process::exit(5);
    }

    if let Err(e) = object_db.generate() {
        println!("Error while generating the data model: {}", e);
        std::process::exit(6);
    }

    if let Err(e) = doc_generator::write_doc(
        &object_db,
        &String::from("./documentation"),
        "tosola_data_model",
    ) {
        println!("Error while generating the documentation: {:?}", e);
        std::process::exit(7);
    }
}
