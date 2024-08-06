use crate::RresFile;

#[test]
fn reads_central_dir() {
    let rres_file = RresFile {
        filename: "examples/resources.rres".into(),
    };
    let central_dir_result = rres_file.load_central_dir();
    match &central_dir_result {
        Ok(central_dir) => assert!(central_dir.entry_count > 0),
        Err(err) => println!("{}", err),
    }
    assert!(central_dir_result.is_ok());
}

#[test]
fn reads_resource_id() {
    let rres_file = RresFile { filename: "examples/resources.rres".into() };
    let central_dir = rres_file.load_central_dir().unwrap();
    let resource_id = central_dir.get_resource_id("resources/text_data.txt".into());
    assert_eq!(resource_id, 3342539433);
}

#[test]
fn reads_resource_chunk() {
    let rres_file = RresFile { filename: "examples/resources.rres".into() };
    let central_dir = rres_file.load_central_dir().unwrap();
    let resource_id = central_dir.get_resource_id("resources/text_data.txt".into());
    let chunk = rres_file.load_resource_chunk(resource_id).unwrap();
    let chunk_string = chunk.data.raw_data.iter().map(|&c| c as char).collect::<String>();
    assert_eq!(chunk_string, "Hello World! This is a test!");
}