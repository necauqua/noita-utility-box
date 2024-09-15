use std::path::PathBuf;

use aes::{
    cipher::{KeyIvInit, StreamCipher},
    Aes128,
};
use anyhow::anyhow;
use clap::Parser;
use ctr::Ctr64BE;

/// A basic symmetric cipher (AES 128bit with 64bit big endian counter) that's used by Noita .salakieli files
#[derive(Debug, clap::Parser)]
struct Args {
    /// The file to encrypt/decrypt
    ///
    /// Since the cipher is symmetric, encryption and decryption are the same operation,
    /// so running this on an encrypted file decrypts it, and running it again on the decrypted one encrypts it.
    input: PathBuf,
    /// Which key to use, in form `key:iv` (without backticks)
    ///
    /// Well known keys are:
    ///   - player.salakieli: `WeSeeATrueSeekerOfKnowledge:YouAreSoCloseToBeingEnlightened`
    ///   - world_state.salakieli: `TheTruthIsThatThereIsNothing:MoreValuableThanKnowledge`
    ///   - magic_numbers.salakieli: `KnowledgeIsTheHighestOfTheHighest:WhoWouldntGiveEverythingForTrueKnowledge`
    ///   - _stats.salakieli, _session.salakieli: `SecretsOfTheAllSeeing:ThreeEyesAreWatchingYou`
    ///
    /// Note that only first 16 characters of key and iv are the actual key/iv,
    /// so you can shorten it to `SecretsOfTheAll:ThreeEyesAreWatc`, for example.
    ///
    /// IV stands for initial vector, usually it's just stored as part of the encrypted file,
    /// but in Noita case the files are just raw ciphertext and the IV is provided separately.
    #[clap(verbatim_doc_comment)]
    key: String,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let (key, iv) = args.key.split_once(':').unwrap_or((&args.key, ""));

    let mut key = escape_bytes::unescape(key.as_bytes()).map_err(|e| anyhow!("{:?}", e))?;
    let mut iv = escape_bytes::unescape(iv.as_bytes()).map_err(|e| anyhow!("{:?}", e))?;

    // make them 128 bit exactly
    key.resize(16, 0);
    iv.resize(16, 0);

    let path = args.input;

    let mut data = std::fs::read(path)?;

    let cipher = &mut Ctr64BE::<Aes128>::new(key.as_slice().into(), iv.as_slice().into());
    cipher.apply_keystream(&mut data);

    let parsed = nxml_rs::parse(std::str::from_utf8(&data)?)?;

    let parsed = parsed.to_owned();

    let wins: u32 = (&parsed / "KEY_VALUE_STATS")
        .children("E")
        .filter(|&child| {
            let key = child % "key";
            key == "progress_ending0" || key == "progress_ending1"
        })
        .map(|child| (child % "value").parse::<u32>().unwrap())
        .sum();

    println!("wins: {:?}", wins);
    // std::io::stdout().lock().write_all(&data)?;

    Ok(())
}
