/**
 * Accepts Base64 encoded Solana transactions.
 *
 * Inputs:
 *    - Solana transaction in Base64 encoded format on stdin
 *    - Private keys from a solana command line compatible json file
 *    - mnemonic and passcode typed into command line
 *
 * Actions:
 *    - Sign transaction with one or more private keys
 *
 * Outputs:
 *    - List of provided/required signatures (before signing)
 *    - Human-readable decoded version of the transaction (before signing)
 *    - Human-readable decoded version of the transaction (after signing)
 *    - Base64 encoded version of the transaction (after signing)
 *    - Signature of the transaction, i.e. the fee payer signature (after signing)
 **/

/* 34567890123456789012345678901234567890123456789012345678901234567890123456789 */
#[rustfmt::skip]
fn usage_string() -> String
{
    "\nUsage: solsign [--help]\n\
            solsign [--no-prompt] [KEY_FILE]...\n\n\
    solsign reads Solana transactions in Base64 encoded format from stdin,\n\
    displays them, signs them, writes signed transactions and signatures to\n\
    stdout.\n\n\
    On start-up, solsign reads any private key files specified on the command\n\
    line. It also prompts for mnemonic and passcode combinations from stdin.\n\
    Collectively these signing keys become available to the program to sign\n\
    transactions.\n\n\
    After reading in private keys, solsign enters a loop where it waits to read\n\
    base64-encoded transactions from standard input. After each encoded\n\
    transaction is read in, the user is prompted to perform any of these actions\n\
    by typing the corresponding letter:\n\n\
    \x20 (,) Repeat a display of the base64-encoded transaction.\n\
    \x20 (.) Display a decoded version of the transaction.\n\
    \x20 (-) Sign the transaction and display the Base64 encoded version of the\n\
    \x20     signed transaction. Note that the transaction may still not be\n\
    \x20     completely signed if not all private keys were available for signing.\n\
    \x20 (=) Sign the transaction and display the base-58 encoded fee payer\n\
    \x20     signature. This option is only available if the transaction can be\n\
    \x20     completely signed using available keys.\n\
    \x20 (/) Clear any partially read transaction from memory. This can be used\n\
    \x20     in case accidental input was provided.\n\n\
    These actions may be repeated for the current transaction until a new\n\
    transaction is input.\n\n\
    On entry of a new transaction, the same options are presented for it.\n\n\
    If --no-prompt is specified, then no prompting will be done. Instead, solsign\n\
    will read the private keys supplied on the command line, read a single\n\
    base64-encoded transaction from standard input, fully sign the transaction,\n\
    write the fee payer signature to standard out, then exit.\n\n\
    Input transactions are expected to be fully-formed Solana transactions,\n\
    with any signatures not yet provided supplied as all zero bytes.  solsign\n\
    identifies signatures that are required by finding all-zero signatures and\n\
    then replacing them with the signature of the transaction as computed using\n\
    the private key corresponding to the public key that should sign the\n\
    transaction at that signature index.  This allows for partial signing of\n\
    transactions, where one signer can sign it, then pass the Base64 encoded form\n\
    of the partially signed transaction to another signer, who can then add their\n\
    own signature, repeating this process until all signatures have been provided\n\
    and the transaction is complete.\n".to_string()
}

fn main()
{
    println!("{}", usage_string())
}
