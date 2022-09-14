use ed25519_dalek::Signer;
use std::collections::HashMap;
use std::io::Write;

/**
 * Accepts Base64 encoded Solana transactions.
 *
 * Inputs:
 *    - Private keys from a solana command line compatible json file
 *    - mnemonic and passcode typed into command line
 *    - Solana transaction in Base64 encoded format on stdin
 *
 * Actions:
 *    - Sign transaction with one or more private keys
 *
 * Outputs:
 *    - Base64 encoded version of the transaction (after signing, if tx still not complete)
 *    - Signature of the transaction, i.e. the fee payer signature (if tx is complete)
 **/


/*      4567890123456789012345678901234567890123456789012345678901234567890123456789 */
#[rustfmt::skip]
fn usage_string() -> String
{
    "\nUsage: solsign [--help]\n\
    \x20      solsign [--no-prompt] [KEY_FILE]...\n\n\
    \x20 solsign reads Solana transactions in Base64 encoded format from stdin,\n\
    \x20 displays them, signs them, writes signed transactions and signatures to\n\
    \x20 stdout.\n\n\
    \x20 On start-up, solsign reads any private key files specified on the command\n\
    \x20 line. It also prompts for mnemonic and passcode combinations from stdin.\n\
    \x20 Collectively these signing keys become available to the program to sign\n\
    \x20 transactions.\n\n\
    \x20 After reading in private keys, solsign asks the user to supply a challenge\n\
    \x20 password which will be used to ensure that the correct user is signing\n\
    \x20 subsequent transactions.  Entering a password is highly recommended as it\n\
    \x20 will protect the user in case an intruder gains access to the command\n\
    \x20 line.\n\n\
    \x20 solsign then enters a loop where it waits to read Base64 encoded\n\
    \x20 transactions from standard input. After each encoded transaction is read\n\
    \x20 in, if there was a challenge password set, solsign will require the user\n\
    \x20 to supply that password before proceeding.  For any signatures not\n\
    \x20 provided within the transaction, if the key required for that signature\n\
    \x20 was provided to solsign, the transaction will be signed with that key.\n\n\
    \x20 After all possible signatures are applied, if the transaction is still not\n\
    \x20 completely signed, then the list of pubkeys which must still sign the\n\
    \x20 transaction is printed, along with the Base64 encoded version of the\n\
    \x20 partially signed transaction is printed, ready for further signing.\n\n\
    \x20 If after signing, the transaction is completely signed, then the signature\n\
    \x20 of the transaction is printed.\n".to_string()
}

// This comes from solana validator code base, which requires all transactions to fit inside an IPV4 UDP packet
// minus some overhead
pub const MAXIMUM_TRANSACTION_BYTES : u16 = 1232;

// (1232 - (4 + 32 + 1) - 1) / 64
pub const MAXIMUM_ED25519_SIGNATURES_COUNT : u8 = 18;

// (1232 - (1 + 32 + 1) - 4) / 32
pub const MAXIMUM_ADDRESSES_COUNT : u8 = 37;

// (1232 - (1 + 4 + 32) - (1 + 1 + 2 + 1))
pub const MAXIMUM_INSTRUCTION_ADDRESS_INDEX_COUNT : u16 = 1190;

// (1232 - (1 + 4 + 32) - 1) - 2
pub const MAXIMUM_INSTRUCTION_DATA_COUNT : u16 = 1192;

// (1232 - (1 + 4 + 32) - 2) / 3
pub const MAXIMUM_INSTRUCTIONS_COUNT : u16 = 397;

#[derive(Clone, PartialEq)]
struct Pubkey(pub [u8; 32]);

#[derive(Clone)]
struct Sha256Digest(pub [u8; 32]);

#[derive(Clone, PartialEq)]
struct Address(pub [u8; 32]);

#[derive(Clone)]
struct PubkeyWithSignature
{
    pub pubkey : Pubkey,

    pub signature : Option<ed25519::Signature>
}

struct Transaction
{
    pub signed_read_write_addresses : Vec<PubkeyWithSignature>,

    pub signed_read_only_addresses : Vec<PubkeyWithSignature>,

    pub unsigned_read_write_addresses : Vec<Address>,

    pub unsigned_read_only_addresses : Vec<Address>,

    pub recent_blockhash : Option<Sha256Digest>,

    pub instructions : Vec<Instruction>
}

struct Instruction
{
    pub program_address : Address,

    // (address, is_signed, is_read_write)
    pub addresses : Vec<(Address, bool, bool)>,

    pub data : Vec<u8>
}

const EMPTY_RECENT_BLOCKHASH : Sha256Digest = Sha256Digest([0_u8; 32]);

const EMPTY_SIGNATURE_BYTES : [u8; 64] = [0_u8; 64];

// To be implemented: read a character in terminal raw mode, i.e. the moment that the user types a character,
// return the typed character.  This prevents having to press return after entering a transaction, or after
// entering a command key.
//
// This is hard to do in a cross-platform (or even single-platform!) manner in Rust, so for the moment, just punt
// and require the return
//
// The returned value is the ASCII representation of a key on a keyboard.
//
// Returns None if stdin() has been closed (i.e. end of input)
//fn read_character() -> Option<u8>
//{
//    let mut line = "".to_string();
//
//    loop {
//        match std::io::stdin().read_line(&mut line) {
//            Ok(_) => {
//                if line.len() == 0 {
//                    // End of input
//                    return None;
//                }
//                let line = line.replace("\n", "").replace("\r", "");
//                if line.len() > 0 {
//                    return Some(line.bytes().nth(0).unwrap());
//                }
//                // Else continue loop
//            },
//
//            Err(_) => return None
//        }
//    }
//}

// The following were all cribbed from solana's code base: sdk/src/signer/keypair.rs:
// keypair_from_seed
// keypair_from_seed_and_derivation_path
// bip32_derived_keypair
// generate_seed_from_seed_phrase_and_passphrase
fn keypair_from_seed(seed : &[u8]) -> Result<ed25519_dalek::Keypair, String>
{
    if seed.len() < ed25519_dalek::SECRET_KEY_LENGTH {
        return Err("Seed is too short".to_string());
    }
    let secret =
        ed25519_dalek::SecretKey::from_bytes(&seed[..ed25519_dalek::SECRET_KEY_LENGTH]).map_err(|e| e.to_string())?;
    let public = ed25519_dalek::PublicKey::from(&secret);
    let dalek_keypair = ed25519_dalek::Keypair { secret, public };
    Ok(dalek_keypair)
}

fn keypair_from_seed_and_derivation_path(
    seed : &[u8],
    derivation_path : derivation_path::DerivationPath
) -> Result<ed25519_dalek::Keypair, String>
{
    bip32_derived_keypair(seed, derivation_path).map_err(|err| err.to_string().into())
}

fn bip32_derived_keypair(
    seed : &[u8],
    derivation_path : derivation_path::DerivationPath
) -> Result<ed25519_dalek::Keypair, String>
{
    let extended = ed25519_dalek_bip32::ExtendedSecretKey::from_seed(seed)
        .and_then(|extended| extended.derive(&derivation_path))
        .map_err(|e| e.to_string())?;
    let extended_public_key = extended.public_key();
    Ok(ed25519_dalek::Keypair { secret : extended.secret_key, public : extended_public_key })
}

fn generate_seed_from_seed_phrase_and_passphrase(
    seed_phrase : &str,
    passphrase : &str
) -> Vec<u8>
{
    const PBKDF2_ROUNDS : u32 = 2048;
    const PBKDF2_BYTES : usize = 64;

    let salt = format!("mnemonic{}", passphrase);

    let mut seed = vec![0u8; PBKDF2_BYTES];
    pbkdf2::pbkdf2::<hmac::Hmac<sha2::Sha512>>(seed_phrase.as_bytes(), salt.as_bytes(), PBKDF2_ROUNDS, &mut seed);
    seed
}

fn print_base64(bytes : &[u8])
{
    let b = base64::encode(&bytes);
    for idx in (0..b.len()).step_by(72) {
        let end = std::cmp::min(idx + 72, b.len());
        println!("    {}", &b[idx..end]);
    }
}

impl Transaction
{
    pub fn decode(r : &mut dyn std::io::Read) -> Result<Self, Option<String>>
    {
        let signatures_count = Self::decode_compact_u16(r)?;

        // Can't provide more signatures than allowed
        if signatures_count > (MAXIMUM_ED25519_SIGNATURES_COUNT as u16) {
            return Err(Some(format!(
                "Too many signatures in transaction: expected at most {}, got {}",
                MAXIMUM_ED25519_SIGNATURES_COUNT, signatures_count
            )));
        }

        let mut signatures = Vec::<Option<ed25519::Signature>>::new();

        let mut buf = [0_u8; 64];

        for _ in 0..signatures_count {
            Self::read(r, &mut buf)?;
            signatures.push(if buf == EMPTY_SIGNATURE_BYTES {
                None
            }
            else {
                Some(ed25519::Signature::from_bytes(&buf).map_err(|e| format!("{}", e))?)
            });
        }

        Self::read(r, &mut buf[0..3])?;

        let total_signed_address_count = buf[0] as u16;

        if total_signed_address_count > (MAXIMUM_ADDRESSES_COUNT as u16) {
            return Err(Some(format!(
                "Too many signatures supplied: expected at most {}, got {}",
                total_signed_address_count, MAXIMUM_ADDRESSES_COUNT
            )));
        }

        // Our encoder always produces all signatures, but uses all zero signatures for those signatures which were
        // not provided.  Other implementations may instead produce a short signatures list, which can only be
        // signatures in order, with unsupplied signatures being zero.
        if signatures_count > total_signed_address_count {
            return Err(Some(format!(
                "Too many signatures supplied: expected at most {}, got {}",
                total_signed_address_count, signatures_count
            )));
        }

        let signed_read_only_address_count = buf[1] as u16;

        if signed_read_only_address_count > total_signed_address_count {
            return Err(Some(format!(
                "Too many signed read only addresses: expected at most {}, got {}",
                total_signed_address_count, signed_read_only_address_count
            )));
        }

        let signed_read_write_address_count = total_signed_address_count - signed_read_only_address_count;

        if signed_read_write_address_count == 0 {
            return Err(Some("Minimum signed address count of 1 required for fee payer".to_string()));
        }

        let unsigned_read_only_address_count = buf[2] as u16;

        let minimum_address_count = total_signed_address_count + unsigned_read_only_address_count;

        let actual_address_count = Self::decode_compact_u16(r)?;

        if actual_address_count < minimum_address_count {
            return Err(Some(format!(
                "Too few addresses in header; {} supplied but at least {} required",
                actual_address_count, minimum_address_count
            )));
        }

        let unsigned_read_write_address_count = actual_address_count - minimum_address_count;

        let mut ret = Transaction {
            signed_read_write_addresses : vec![],
            signed_read_only_addresses : vec![],
            unsigned_read_write_addresses : vec![],
            unsigned_read_only_addresses : vec![],
            recent_blockhash : None,
            instructions : vec![]
        };

        let mut signatures_iter = signatures.into_iter();

        for _ in 0..signed_read_write_address_count {
            ret.signed_read_write_addresses.push(Self::decode_signature_from_header(&mut signatures_iter, r)?);
        }

        for _ in 0..signed_read_only_address_count {
            ret.signed_read_only_addresses.push(Self::decode_signature_from_header(&mut signatures_iter, r)?);
        }

        for _ in 0..unsigned_read_write_address_count {
            ret.unsigned_read_write_addresses.push(Self::decode_address(r)?);
        }

        for _ in 0..unsigned_read_only_address_count {
            ret.unsigned_read_only_addresses.push(Self::decode_address(r)?);
        }

        ret.recent_blockhash = Self::decode_recent_blockhash(r)?;

        let instruction_count = Self::decode_compact_u16(r)?;

        for i in 0..instruction_count {
            let i = i as usize;
            Self::read(r, &mut buf[0..1])?;

            let program_address = ret
                .find_address_at_index(buf[0])
                .ok_or(format!("Invalid program id index {} for instruction {}", buf[0], i))?;

            let addresses_count = Self::decode_compact_u16(r)?;

            if addresses_count > MAXIMUM_INSTRUCTION_ADDRESS_INDEX_COUNT {
                return Err(Some(format!(
                    "Too many addresses in instruction {}: expected at most {} got {}",
                    i, MAXIMUM_INSTRUCTION_ADDRESS_INDEX_COUNT, addresses_count
                )));
            }

            let mut addresses = Vec::<(Address, bool, bool)>::new();

            for _ in 0..addresses_count {
                Self::read(r, &mut buf[0..1])?;
                addresses.push(
                    ret.find_address_at_index(buf[0])
                        .ok_or(format!("Invalid address index {} referenced from instruction {}", buf[0], i))?
                );
            }

            let data_count = Self::decode_compact_u16(r)?;

            if data_count > MAXIMUM_INSTRUCTION_DATA_COUNT {
                return Err(Some(format!(
                    "Too many data bytes in instruction {}: expected at most {} got {}",
                    i, MAXIMUM_INSTRUCTION_DATA_COUNT, data_count
                )));
            }

            let mut data = vec![0_u8; data_count as usize];

            Self::read(r, &mut data)?;

            ret.instructions.push(Instruction { program_address : program_address.0, addresses, data });
        }

        Ok(ret)
    }

    // Return the message bytes of the transaction.
    pub fn message(
        &self,
        w : &mut dyn std::io::Write
    ) -> Result<(), String>
    {
        u8::try_from(self.signed_read_write_addresses.len() + self.signed_read_only_addresses.len())
            .or(Err("Too many signed addresses".to_string()))
            .and_then(|u| Self::write(w, &[u]))?;

        u8::try_from(self.signed_read_only_addresses.len())
            .or(Err("Too many read only addresses".to_string()))
            .and_then(|u| Self::write(w, &[u]))?;

        Self::write(w, &[self.unsigned_read_only_addresses.len() as u8])?;

        let recent_blockhash = self.recent_blockhash.as_ref().unwrap_or(&EMPTY_RECENT_BLOCKHASH);

        if self.instructions.len() > (u16::MAX as usize) {
            return Err("Too many instructions".to_string());
        }

        // compact-array of account addresses
        Self::encode_compact_u16(
            (self.signed_read_write_addresses.len() +
                self.signed_read_only_addresses.len() +
                self.unsigned_read_write_addresses.len() +
                self.unsigned_read_only_addresses.len()) as u16,
            w
        )?;

        for a in self
            .signed_read_write_addresses
            .iter()
            .chain(self.signed_read_only_addresses.iter())
            .map(|s| &s.pubkey.0)
            .chain(
                self.unsigned_read_write_addresses.iter().chain(self.unsigned_read_only_addresses.iter()).map(|a| &a.0)
            )
        {
            Self::write(w, a)?;
        }

        // recent blockhash
        Self::write(w, &recent_blockhash.0)?;

        // instructions
        Self::encode_compact_u16(self.instructions.len() as u16, w)?;

        for instruction in &self.instructions {
            // instruction program_id index
            Self::write(
                w,
                std::slice::from_ref(&self.find_address_index(&instruction.program_address).ok_or(format!(
                    "Invalid Transaction - program address {} not in address list",
                    instruction.program_address
                ))?)
            )?;

            // instruction address indices
            Self::encode_compact_u16(instruction.addresses.len() as u16, w)?;
            for a in &instruction.addresses {
                Self::write(
                    w,
                    std::slice::from_ref(
                        &self
                            .find_address_index(&a.0)
                            .ok_or(format!("Invalid Transaction - address {} is not in address list", a.0))?
                    )
                )?;
            }

            // instruction data
            let data_len = instruction.data.len();
            if data_len > (MAXIMUM_INSTRUCTION_DATA_COUNT as usize) {
                return Err(format!(
                    "Instruction data len too long: {} > {}",
                    data_len, MAXIMUM_INSTRUCTION_DATA_COUNT
                ));
            }
            Self::encode_compact_u16(data_len as u16, w)?;
            Self::write(w, instruction.data.as_slice())?;
        }
        Ok(())
    }

    // Iterates over addresses that still need to provide a signature
    pub fn needed_signatures(&self) -> impl Iterator<Item = Pubkey>
    {
        let mut v : Vec<Pubkey> = self
            .signed_read_write_addresses
            .iter()
            .filter_map(|a| {
                if a.signature.is_some() {
                    None
                }
                else {
                    Some(a.pubkey.clone())
                }
            })
            .chain(self.signed_read_only_addresses.iter().filter_map(|a| {
                if a.signature.is_some() {
                    None
                }
                else {
                    Some(a.pubkey.clone())
                }
            }))
            .collect();

        v.sort_by_key(|a| format!("{}", a));

        v.dedup();

        v.into_iter()
    }

    pub fn sign(
        &mut self,
        pubkey : &Pubkey,
        signature : ed25519::Signature
    ) -> Result<(), String>
    {
        for i in 0..self.signed_read_write_addresses.len() {
            if self.signed_read_write_addresses[i].pubkey == *pubkey {
                self.signed_read_write_addresses[i].signature = Some(signature);
            }
        }

        for i in 0..self.signed_read_only_addresses.len() {
            if self.signed_read_only_addresses[i].pubkey == *pubkey {
                self.signed_read_only_addresses[i].signature = Some(signature);
            }
        }

        Ok(())
    }

    pub fn encode(
        &self,
        w : &mut dyn std::io::Write
    ) -> Result<(), String>
    {
        let total_signatures = self.signed_read_write_addresses.len() + self.signed_read_only_addresses.len();

        if total_signatures > (u16::MAX as usize) {
            return Err("Too many addresses".to_string());
        }

        Self::encode_compact_u16(total_signatures as u16, w)?;

        for signature in self.signed_read_write_addresses.iter().chain(&self.signed_read_only_addresses) {
            Self::encode_signature(signature.signature, w)?;
        }

        self.message(w)
    }

    fn decode_compact_u16(r : &mut dyn std::io::Read) -> Result<u16, Option<String>>
    {
        let mut buf = [0_u8; 3];

        Self::read(r, &mut buf[0..1])?;

        if (buf[0] & 0x80) == 0x80 {
            Self::read(r, &mut buf[1..2])?;
            if buf[1] & 0x80 == 0x80 {
                Self::read(r, &mut buf[2..3])?;
                Ok((((buf[0] as u16) & !0x80) << 0) |
                    (((buf[1] as u16) & !0x80) << 7) |
                    (((buf[2] as u16) & !0x00) << 14))
            }
            else {
                Ok((((buf[0] as u16) & !0x80) << 0) | (((buf[1] as u16) & !0x80) << 7))
            }
        }
        else {
            Ok(buf[0] as u16)
        }
    }

    fn decode_signature_from_header(
        signatures : impl IntoIterator<Item = Option<ed25519::Signature>>,
        r : &mut dyn std::io::Read
    ) -> Result<PubkeyWithSignature, Option<String>>
    {
        let address = Self::decode_address(r)?;

        Ok(PubkeyWithSignature {
            pubkey : Pubkey(address.0),
            signature : signatures.into_iter().next().unwrap_or(None)
        })
    }

    fn decode_address(r : &mut dyn std::io::Read) -> Result<Address, Option<String>>
    {
        let mut buf = [0_u8; 32];
        Self::read(r, &mut buf)?;
        Ok(Address(buf))
    }

    fn decode_recent_blockhash(r : &mut dyn std::io::Read) -> Result<Option<Sha256Digest>, Option<String>>
    {
        let mut buf = [0_u8; 32];

        Self::read(r, &mut buf)?;

        if buf == EMPTY_RECENT_BLOCKHASH.0 {
            Ok(None)
        }
        else {
            Ok(Some(Sha256Digest(buf)))
        }
    }

    // Searching is done irrespective of account permissions.  This matches the expected Solana runtime behavior,
    // where the execution system will perform a similar action.  It is technically possible to encode the same
    // address with multiple permissions versions, but the runtime will reject such a transaction with an error about
    // "Account loaded twice"
    fn find_address_index(
        &self,
        address : &Address
    ) -> Option<u8>
    {
        match self.signed_read_write_addresses.iter().position(|s| address == &s.pubkey) {
            Some(index) => return Some(index as u8),
            None => ()
        }

        let mut offset = self.signed_read_write_addresses.len();

        match self.signed_read_only_addresses.iter().position(|s| address == &s.pubkey) {
            Some(index) => return Some((index + offset) as u8),
            None => ()
        }

        offset += self.signed_read_only_addresses.len();

        match self.unsigned_read_write_addresses.iter().position(|a| address == a) {
            Some(index) => return Some((index + offset) as u8),
            None => ()
        }

        offset += self.unsigned_read_write_addresses.len();

        match self.unsigned_read_only_addresses.iter().position(|a| address == a) {
            Some(index) => Some((index + offset) as u8),
            None => None
        }
    }

    // Returns (address, is_signed, read_write)
    fn find_address_at_index(
        &self,
        index : u8
    ) -> Option<(Address, bool, bool)>
    {
        let mut uindex = index as usize;

        if uindex < self.signed_read_write_addresses.len() {
            return Some((Address(self.signed_read_write_addresses[uindex].pubkey.0), true, true));
        }

        uindex -= self.signed_read_write_addresses.len();

        if uindex < self.signed_read_only_addresses.len() {
            return Some((Address(self.signed_read_only_addresses[uindex].pubkey.0), true, false));
        }

        uindex -= self.signed_read_only_addresses.len();

        if uindex < self.unsigned_read_write_addresses.len() {
            return Some((Address(self.unsigned_read_write_addresses[uindex].0), false, true));
        }

        uindex -= self.unsigned_read_write_addresses.len();

        if uindex < self.unsigned_read_only_addresses.len() {
            return Some((Address(self.unsigned_read_only_addresses[uindex].0), false, false));
        }

        None
    }

    fn encode_compact_u16(
        mut u : u16,
        w : &mut dyn std::io::Write
    ) -> Result<(), String>
    {
        let mut buf = [0_u8; 3];

        let mut v = (u & 0x7F) as u8;
        if u > 0x7F {
            buf[0] = v | 0x80;
            u >>= 7;
            v = (u & 0x7F) as u8;
            if u > 0x7F {
                buf[1] = v | 0x80;
                buf[2] = (u >> 7) as u8;
                Self::write(w, &buf)
            }
            else {
                buf[1] = v;
                Self::write(w, &buf[0..2])
            }
        }
        else {
            buf[0] = v;
            Self::write(w, &buf[0..1])
        }
    }

    fn encode_signature(
        signature : Option<ed25519::Signature>,
        w : &mut dyn std::io::Write
    ) -> Result<(), String>
    {
        Self::write(w, signature.map(|s| s.to_bytes()).unwrap_or(EMPTY_SIGNATURE_BYTES).as_slice())
    }

    fn read(
        r : &mut dyn std::io::Read,
        buf : &mut [u8]
    ) -> Result<(), Option<String>>
    {
        match r.read_exact(buf) {
            Ok(_) => Ok(()),

            Err(e) => match e.kind() {
                std::io::ErrorKind::UnexpectedEof => Err(None),

                _ => Err(Some(e.to_string()))
            }
        }
    }

    fn write(
        w : &mut dyn std::io::Write,
        buf : &[u8]
    ) -> Result<(), String>
    {
        w.write_all(&buf).map(|_| ()).map_err(|e| format!("{}", e))
    }
}

fn main()
{
    let mut no_prompt = false;

    // This is a map from base-58 encoded public key to key
    let mut keys = HashMap::<String, ed25519_dalek::Keypair>::new();

    let mut keys_in_order = Vec::<String>::new();

    let mut key_files = Vec::<String>::new();

    key_files.extend(std::env::args().skip(1));

    if key_files.len() > 0 {
        match key_files[0].as_str() {
            "--help" => {
                println!("{}", usage_string());
                std::process::exit(0);
            },

            "--no-prompt" => {
                no_prompt = true;
                key_files.remove(0);
            },

            _ => ()
        }
    }

    for key_file in key_files {
        let contents : String = std::fs::read_to_string(&key_file).unwrap_or_else(|e| {
            eprintln!("\nERROR: Failed to read key file {}: {}\n", key_file, e);
            std::process::exit(-1);
        });

        // Strip whitespace and [ and ], split by , and then parse bytes
        let private_key_bytes : Vec<u8> = contents
            .replace(" ", "")
            .replace("[", "")
            .replace("]", "")
            .split(",")
            .map(|s| {
                u8::from_str_radix(s, 10).unwrap_or_else(|e| {
                    eprintln!("\nERROR: Invalid key file value {}: {}\n", key_file, e);
                    std::process::exit(-1);
                })
            })
            .collect();

        let dalek_keypair = ed25519_dalek::Keypair::from_bytes(private_key_bytes.as_slice()).unwrap_or_else(|e| {
            eprintln!("\nERROR: Invalid private key file {}: {}\n", key_file, e);
            std::process::exit(-1);
        });

        let public_key = bs58::encode(dalek_keypair.public.to_bytes()).into_string();

        if keys.insert(public_key.clone(), dalek_keypair).is_none() {
            keys_in_order.push(public_key);
        }
    }

    // If no-prompt, don't read keys in from stdin
    if !no_prompt {
        loop {
            println!("\n  Public keys provided thus far:\n");

            if keys_in_order.len() == 0 {
                println!("    None");
            }
            else {
                for key in &keys_in_order {
                    println!("    {}", key);
                }
            }

            let mnemonic =
                rpassword::prompt_password("\n  Enter mnemonic seed words of next key, or press ENTER to continue: ")
                    .unwrap_or_else(|_| {
                        println!("\n");
                        std::process::exit(0);
                    });

            if mnemonic.len() == 0 {
                break;
            }

            let mnemonic = mnemonic.trim();

            let passphrase =
                rpassword::prompt_password("\n  Enter passphrase seed, or press ENTER for no passphrase: ")
                    .unwrap_or_else(|_| {
                        println!("\n");
                        std::process::exit(0);
                    });

            let seed = generate_seed_from_seed_phrase_and_passphrase(&mnemonic, &passphrase);

            // Now derive keypairs directly, and with derivation path m/44'/501'/0'/0' through m/44'/501'/0'/9', to
            // cover all expected possible sources of mnemonics and passphrases (i.e. solana-keygen plus standard
            // wallets).  Then let the user choose which was their key (or none!).
            let mut keypairs = Vec::<(String, ed25519_dalek::Keypair)>::new();

            keypairs.push((
                "                ".to_string(),
                keypair_from_seed(&seed).unwrap_or_else(|e| {
                    eprintln!("\n{}\n", e);
                    std::process::exit(-1);
                })
            ));

            for i in 0..9 {
                let mut path = Vec::<derivation_path::ChildIndex>::new();
                path.push(derivation_path::ChildIndex::Hardened(44));
                path.push(derivation_path::ChildIndex::Hardened(501));
                path.push(derivation_path::ChildIndex::Hardened(0));
                path.push(derivation_path::ChildIndex::Hardened(i));
                let derivation_path = derivation_path::DerivationPath::new(&*path);
                keypairs.push((
                    format!("m/44'/501'/0'/{}'", i),
                    keypair_from_seed_and_derivation_path(&seed, derivation_path).unwrap_or_else(|e| {
                        eprintln!("\n{}\n", e);
                        std::process::exit(-1);
                    })
                ));
            }

            loop {
                println!("\n  Derived Keys:\n");

                for i in 0..keypairs.len() {
                    let kp = &keypairs[i];
                    let padding = if i > 9 { " ".to_string() } else { "  ".to_string() };
                    println!("   ({}){}{}    {}", i, padding, kp.0, bs58::encode(kp.1.public.to_bytes()).into_string());
                }

                print!("\n  Select a derived key 0 - 9 from above, or press ENTER to skip: ");
                let _ = std::io::stdout().flush();

                let mut line = "".to_string();
                std::io::stdin().read_line(&mut line).unwrap_or_else(|_| {
                    std::process::exit(0);
                });

                if line.len() == 0 {
                    println!("\n");
                    std::process::exit(0);
                }

                let line = line.replace("\n", "").replace("\r", "");

                if line.len() == 0 {
                    break;
                }

                if let Ok(selection) = u8::from_str_radix(&line, 10).map(|s| s as usize) {
                    if selection < keypairs.len() {
                        let kp = keypairs.remove(selection).1;
                        let public_key = bs58::encode(kp.public.to_bytes()).into_string();
                        if keys.insert(public_key.clone(), kp).is_none() {
                            keys_in_order.push(public_key);
                        }
                        break;
                    }
                    else {
                        println!("\n\n  Invalid selection, try again.\n");
                    }
                }
                else {
                    println!("\n\n  Invalid selection, try again.\n");
                }
            }
        }
    }

    println!("");

    if keys_in_order.len() == 0 {
        eprintln!("  No keys provided, cannot sign.  Exiting.\n");
        std::process::exit(-1);
    }

    // Allow the user to provide a password that will be used to challenge them before each transaction is signed.
    // This improves security - in case the user steps away from their computer, no one else can sign transactions if
    // they don't know the password
    let password = if no_prompt {
        "".to_string()
    }
    else {
        rpassword::prompt_password(
            "  Enter a password to be challenged with before each transaction is signed\n  or press ENTER for no \
             signing challenge password: "
        )
        .unwrap_or_else(|_| {
            println!("\n");
            std::process::exit(0);
        })
    };

    loop {
        println!("\n  Enter Base64 encoded transaction:\n");

        // Read lines until a complete transaction is read in
        let mut tx = "".to_string();
        loop {
            let mut line = "".to_string();
            std::io::stdin().read_line(&mut line).unwrap_or_else(|_| {
                println!("\n");
                std::process::exit(0);
            });

            if line.len() == 0 {
                println!("\n");
                std::process::exit(0);
            }

            line.retain(|c| !c.is_whitespace());

            if line.len() > 0 {
                tx.push_str(&line);
            }

            // Attempt a decode.  Might be short because not all lines of the transaction have been provided yet.

            // Decode Base64
            match base64::decode(&tx) {
                Ok(bytes) => {
                    match Transaction::decode(&mut bytes.as_slice()) {
                        // If a completely decoded transaction was found, sign it
                        Ok(mut decoded_tx) => {
                            // Get the transaction to sign -- everything except the signatures
                            let mut message = vec![];
                            decoded_tx.message(&mut message).unwrap_or_else(|e| {
                                eprintln!("\n{}\n", e);
                                std::process::exit(-1);
                            });

                            if password.len() > 0 {
                                println!("\n");
                                let mut attempts = 0;
                                loop {
                                    let prompt = format!(
                                        "  Enter challenge password ({} attempt{} remaining): ",
                                        (5 - attempts),
                                        if attempts == 4 { "" } else { "s" }
                                    );
                                    let password_attempt = rpassword::prompt_password(prompt).unwrap_or_else(|_| {
                                        println!("\n");
                                        std::process::exit(0);
                                    });

                                    if password_attempt == password {
                                        println!("");
                                        break;
                                    }

                                    if attempts == 4 {
                                        println!("\n  Password challenge failed.\n");
                                        std::process::exit(0);
                                    }

                                    attempts += 1;
                                }
                            }

                            // For every signature incomplete within the transaction, add that signature if we
                            // have the key, otherwise, put the bs58 encoded key in here.
                            let mut unsigned = Vec::<String>::new();
                            decoded_tx.needed_signatures().for_each(|pubkey| {
                                let pubkey_string = pubkey.to_string();
                                match keys.get(&pubkey_string) {
                                    Some(keypair) => {
                                        decoded_tx.sign(&pubkey, keypair.sign(&message)).unwrap_or_else(|e| {
                                            eprintln!("\nFailed to sign with key {}: {}\n", pubkey_string, e);
                                            std::process::exit(-1);
                                        })
                                    },
                                    None => unsigned.push(pubkey_string)
                                }
                            });

                            // Now output
                            let mut encoded_tx = vec![];
                            match decoded_tx.encode(&mut encoded_tx) {
                                Ok(()) => {
                                    // Now, if the transaction is completely signed, emit the signature
                                    if unsigned.len() == 0 {
                                        if let Some(signature) = decoded_tx.signed_read_write_addresses[0].signature {
                                            println!("\n  Transaction is complete:\n");
                                            print_base64(&encoded_tx);
                                            println!(
                                                "\n  Signature:\n\n   {}",
                                                bs58::encode(signature.to_bytes()).into_string()
                                            );
                                        }
                                    }
                                    // Else, emit the partially signed tx
                                    else {
                                        println!("\n  Pubkeys still needed to sign:");
                                        unsigned.iter().for_each(|pubkey| println!("\n    {}", pubkey));
                                        println!("\n  Partially signed transaction:\n");
                                        print_base64(&encoded_tx);
                                    }
                                },
                                Err(e) => eprintln!("\n{}\n", e)
                            }

                            break;
                        },

                        // If an error occurred, then input was bad; break the loop to get the next tx
                        Err(Some(_)) => break,

                        // The only other possibility is Err(None) => incomplete data, so continue reading lines
                        Err(None) => ()
                    }
                },
                Err(e) => {
                    if line.len() == 0 {
                        eprintln!("  Invalid Base64 input: {}", e);
                        eprintln!("\n  Clearing tx data, start again.");
                        break;
                    }
                },
            }
        }

        // no_prompt stops after the first transaction
        if no_prompt {
            println!("");
            break;
        }
    }
}

impl std::fmt::Display for Pubkey
{
    fn fmt(
        &self,
        f : &mut std::fmt::Formatter
    ) -> std::fmt::Result
    {
        write!(f, "{}", bs58::encode(self.0).into_string())
    }
}

impl std::fmt::Display for Address
{
    fn fmt(
        &self,
        f : &mut std::fmt::Formatter
    ) -> std::fmt::Result
    {
        write!(f, "{}", bs58::encode(self.0).into_string())
    }
}

impl PartialEq<Pubkey> for Address
{
    fn eq(
        &self,
        other : &Pubkey
    ) -> bool
    {
        self.0 == other.0
    }
}
