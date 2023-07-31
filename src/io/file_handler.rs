use std::io::SeekFrom;

use crate::{Result, io::sha256_digest};
use async_std::{
    fs::File,
    io::{prelude::SeekExt, ReadExt, WriteExt},
    path::Path,
};
use log::{info, debug};

#[derive(Debug)]
pub struct FileHandler {
    root: String,
}

impl FileHandler {
    pub fn new(root: String) -> Self {
        FileHandler { root }
    }
}

impl FileHandler {
    ///
    /// Create a file in the root folder
    /// # Arguments
    /// * `root_folder` - The root folder where the file will be created
    /// * `file_name` - The relative path to the root folder and name of the file to be created
    ///
    pub async fn create_file(&self, file_name: String, sha: String) -> Result<()> {
        let path = Path::new(&self.root).join(file_name);
        if path.exists().await {
            let asyn_buf = async_std::path::PathBuf::from(path.clone());
            let existing_sha = crate::io::sha(&asyn_buf).await;
            if let Some(existing_sha) =  existing_sha {
                if existing_sha.eq(&sha) {
                    debug!("File alaready exisits in with the same SHA {:?}",sha);
                    return Ok(())
                }
            }
        }
        let new_file = File::create(path).await?;
        debug!("New file created {:?}",new_file);
        Ok(())
    }

    ///
    ///    Create a folder in the root folder
    ///    # Arguments
    ///    * `root_folder` - The root folder where the folder will be created
    ///    * `folder_name` - The relative path to the root folder and name of the folder to be created
    ///
    pub async fn create_folder(&self, folder_name: String) -> Result<()> {
        let path = Path::new(&self.root).join(folder_name);
        async_std::fs::create_dir_all(path).await?;

        Ok(())
    }

    ///
    ///    Delete a file in the root folder
    ///    # Arguments
    ///    * `root_folder` - The root folder where the file will be deleted
    ///    * `file_name` - The relative path to the root folder and name of the file to be deleted
    ///
    pub async fn delete_file(&self, file_name: String) -> Result<()> {
        let path = Path::new(&self.root).join(file_name);
        async_std::fs::remove_file(path).await?;

        Ok(())
    }

    ///
    ///    Delete a folder in the root folder
    ///    # Arguments
    ///    * `root_folder` - The root folder where the folder will be deleted
    ///    * `folder_name` - The relative path to the root folder and name of the folder to be deleted
    ///    
    pub async fn delete_folder(&self, folder_name: String) -> Result<()> {
        let path = Path::new(&self.root).join(folder_name);
        async_std::fs::remove_dir_all(path).await?;

        Ok(())
    }

    ///
    ///        Writes a random chunk of data to a file
    ///   # Arguments
    ///        * `root_folder` - The root folder where the file will be written
    ///        * `file_name` - The relative path to the root folder and name of the file to be written
    ///        * `offset` - The offset from where to start writing
    ///        * `buf` - The buffer where the data will be written
    ///
    pub async fn write_random(&self, file_name: String, offset: u64, buf: &[u8]) -> Result<()> {
        let path = Path::new(&self.root).join(file_name);
        let mut file = File::open(path).await?;
        file.seek(SeekFrom::Start(offset)).await?;
        file.write(buf).await?;
        Ok(())
    }

    ///
    ///        Reads a random chunk of data from a file and return true if reached EOF
    ///        # Arguments
    ///        * `root_folder` - The root folder where the file will be read
    ///        * `file_name` - The relative path to the root folder and name of the file to be read
    ///        * `offset` - The offset from where to start reading
    ///        * `buf` - The buffer where the data will be read
    ///
    ///        # Returns
    ///        * `bool` - True if reached EOF
    ///
    pub async fn read_random(
        &self,
        file_name: String,
        offset: u64,
        buf: &mut [u8],
    ) -> Result<bool> {
        let path = Path::new(&self.root).join(file_name);
        let mut file = File::open(path).await?;
        file.seek(SeekFrom::Start(offset)).await?;
        let read_data = file.read(buf).await?;

        Ok(read_data > 0)
    }
}
