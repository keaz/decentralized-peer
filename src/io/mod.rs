use async_std::{
    fs::File,
    io::{BufReader, ReadExt},
    path::PathBuf,
};
use data_encoding::HEXUPPER;
use ring::digest::{Context, Digest, SHA256};

pub mod file_handler;
pub mod watch;

async fn sha256_digest(mut reader: BufReader<File>) -> Option<Digest> {
    let mut context = Context::new(&SHA256);
    let mut buffer = [0; 1024];

    loop {
        let count = reader.read(&mut buffer).await.unwrap();
        if count == 0 {
            break;
        }
        context.update(&buffer[..count]);
    }

    Some(context.finish())
}

pub async fn sha(path: &PathBuf) -> Option<String> {
    let input = File::open(path).await.unwrap();
    let reader = BufReader::new(input);
    let digest = sha256_digest(reader).await?;

    std::option::Option::Some(HEXUPPER.encode(digest.as_ref()))
}
