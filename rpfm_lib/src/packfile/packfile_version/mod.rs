use super::*;
use serde_json::to_string_pretty;

use std::fs::File;
use std::io::{BufReader, BufWriter, SeekFrom, Read, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use rpfm_error::{ErrorKind, Result};

use crate::SETTINGS;
use crate::common::{decoder::Decoder, encoder::Encoder};
use crate::packedfile::PackedFileType;

mod pfh0;
mod pfh2;
mod pfh3;
mod pfh4;
mod pfh5;
mod pfh6;

impl PackFile {

    /// This function reads the content of a PackFile into a `PackFile` struct.
    pub fn read(
        file_path: &PathBuf,
        types_to_load: &Option<Vec<PackedFileType>>,
        use_lazy_loading: bool
    ) -> Result<Self> {

        // Check if what we received is even a `PackFile`.
        if !file_path.file_name().unwrap().to_string_lossy().to_string().ends_with(".pack") { return Err(ErrorKind::OpenPackFileInvalidExtension.into()) }

        // Prepare the PackFile to be read and the virtual PackFile to be written.
        let mut pack_file = BufReader::new(File::open(&file_path)?);
        let pack_file_name = file_path.file_name().unwrap().to_string_lossy().to_string();
        let mut pack_file_decoded = Self::new();

        // First, we do some quick checkings to ensure it's a valid PackFile.
        // 24 is the bare minimum that we need to check how a PackFile should be internally, so any file with less than that is not a valid PackFile.
        let pack_file_len = pack_file.get_ref().metadata()?.len();
        if pack_file_len < 24 { return Err(ErrorKind::PackFileHeaderNotComplete.into()) }

        // Create a little buffer to read the basic data from the header of the PackFile.
        let mut buffer = vec![0; 24];
        pack_file.read_exact(&mut buffer)?;

        // Start populating our decoded PackFile struct.
        pack_file_decoded.file_path = file_path.to_path_buf();
        pack_file_decoded.pfh_version = PFHVersion::get_version(&buffer.decode_string_u8(0, 4)?)?;
        pack_file_decoded.pfh_file_type = PFHFileType::get_type(buffer.decode_integer_u32(4)? & 15);
        pack_file_decoded.bitmask = PFHFlags::from_bits_truncate(buffer.decode_integer_u32(4)? & !15);

        // Depending on the data we got, prepare to read the header and ensure we have all the bytes we need.
        match pack_file_decoded.pfh_version {
            PFHVersion::PFH6 => pack_file_decoded.read_pfh6(pack_file, types_to_load, use_lazy_loading)?,
            PFHVersion::PFH5 => pack_file_decoded.read_pfh5(pack_file, types_to_load, use_lazy_loading)?,
            PFHVersion::PFH4 => pack_file_decoded.read_pfh4(pack_file, types_to_load, use_lazy_loading)?,
            PFHVersion::PFH3 => pack_file_decoded.read_pfh3(pack_file, types_to_load, use_lazy_loading)?,
            PFHVersion::PFH2 => pack_file_decoded.read_pfh2(pack_file, types_to_load, use_lazy_loading)?,
            PFHVersion::PFH0 => pack_file_decoded.read_pfh0(pack_file, types_to_load, use_lazy_loading)?,
        }

        Ok(pack_file_decoded)
    }

    /// This function tries to save a `PackFile` to a file in the filesystem.
    ///
    /// If no path is passed, the `PackFile` will be saved in his current path.
    /// If a path is passed as `new_path` the `PackFile` will be saved in that path.
    pub fn save(&mut self, new_path: Option<PathBuf>) -> Result<()> {
        match self.pfh_version {

            // PFH6 contains a subheader with some extra data we want to keep.
            //PFHVersion::PFH6 => pack_file_decoded.read_pfh6(types_to_load, use_lazy_loading),
            //PFHVersion::PFH5 => pack_file_decoded.read_pfh5(types_to_load, use_lazy_loading),
            //PFHVersion::PFH4 => pack_file_decoded.read_pfh4(types_to_load, use_lazy_loading),
            //PFHVersion::PFH3 => pack_file_decoded.read_pfh3(types_to_load, use_lazy_loading),
            PFHVersion::PFH6 => self.save_pfh6(new_path),
            PFHVersion::PFH5 => self.save_pfh5(new_path),
            PFHVersion::PFH4 => self.save_pfh4(new_path),
            PFHVersion::PFH3 => self.save_pfh3(new_path),
            PFHVersion::PFH2 => self.save_pfh2(new_path),
            PFHVersion::PFH0 => self.save_pfh0(new_path),
        }
    }
}
