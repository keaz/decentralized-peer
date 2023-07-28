use std::io::SeekFrom;

use async_std::{path::Path, fs::File, io::{prelude::SeekExt, WriteExt, ReadExt}};
use log::info;
use crate::Result;

///
/// Create a file in the root folder
/// # Arguments
/// * `root_folder` - The root folder where the file will be created
/// * `file_name` - The relative path to the root folder and name of the file to be created
///
pub async fn create_file(root_folder:String, file_name:String) -> Result<()> {
    let path = Path::new(&root_folder).join(file_name);
    let _file = File::create(path).await?;
    
    Ok(())
}

#[doc = r#"
    Create a folder in the root folder
    # Arguments
    * `root_folder` - The root folder where the folder will be created
    * `folder_name` - The relative path to the root folder and name of the folder to be created
"#]
pub async fn create_folder(root_folder:String, folder_name:String) -> Result<()> {
    let path = Path::new(&root_folder).join(folder_name);
    async_std::fs::create_dir_all(path).await?;
    
    Ok(())
}

#[doc = r#"
    Delete a file in the root folder
    # Arguments
    * `root_folder` - The root folder where the file will be deleted
    * `file_name` - The relative path to the root folder and name of the file to be deleted
"#]
pub async fn delete_file(root_folder:String, file_name:String) -> Result<()> {
    let path = Path::new(&root_folder).join(file_name);
    async_std::fs::remove_file(path).await?;
    
    Ok(())
}

#[doc = r#"
    Delete a folder in the root folder
    # Arguments
    * `root_folder` - The root folder where the folder will be deleted
    * `folder_name` - The relative path to the root folder and name of the folder to be deleted
"#]
pub async fn delete_folder(root_folder:String, folder_name:String) -> Result<()> {
    let path = Path::new(&root_folder).join(folder_name);
    async_std::fs::remove_dir_all(path).await?;
    
    Ok(())
}

#[doc = r#"
    Writes a random chunk of data to a file
    # Arguments
    * `root_folder` - The root folder where the file will be written
    * `file_name` - The relative path to the root folder and name of the file to be written
    * `offset` - The offset from where to start writing
    * `buf` - The buffer where the data will be written
"#]
pub async fn write_random(root_folder:String,file_name:String, offset: u64, buf: &[u8]) -> Result<()> {
    let path = Path::new(&root_folder).join(file_name);
    let mut file = File::open(path).await?;
    file.seek(SeekFrom::Start(offset)).await?;
    file.write(buf).await?;
    Ok(())
}

#[doc = r#"
    Reads a random chunk of data from a file and return true if reached EOF
    # Arguments
    * `root_folder` - The root folder where the file will be read
    * `file_name` - The relative path to the root folder and name of the file to be read
    * `offset` - The offset from where to start reading
    * `buf` - The buffer where the data will be read

    # Returns
    * `bool` - True if reached EOF
"#]
pub async fn read_random(root_folder:String,file_name:String, offset: u64, buf: &mut [u8]) -> Result<bool> {
    let path = Path::new(&root_folder).join(file_name);
    let mut file = File::open(path).await?;
    file.seek(SeekFrom::Start(offset)).await?;
    let read_data = file.read(buf).await?;

    Ok(read_data > 0)
}



