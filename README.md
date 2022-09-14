## solsign -- A utility for signing solana transactions off-line ##

# Introduction #

solsign is a small utility program that facilitates signing solana transactions off-line.  ***Off-line signing***
means using keys that are stored locally on the user's computer to sign transactions without connection to any
"wallet" software.

solsign reads complete solana transactions that have been encoded in Base64 format.  Base64 allows the binary data
of a solana transaction to be presented in only text characters suitable for copying and pasting into a terminal
window.  solsign reads transactions, signs them, and then either outputs the newly signed transaction (also in
Base64 format), and also outputs the signature of the transaction if the transaction is now completely signed.

solsign supports a few common workflows:

1. **Signing a single-signer transaction**: this is accomplished by passing the `--no-prompt` argument to solsign, along with the path to a key file.  solsign reads a transaction to sign from standard input, then signs it and outputs the signed version.

2. **Repeatedly signing single-signer transactions**: this is accomplished by passing the path to a key file to solsign on the command-line, **or** inputting the mnemonic and passphrase to solsign after it has started up.  After that, solsign will read transactions from standard input, sign them, and write the signatures to standard output, continuing to do so until the user ends the input.  This would be useful for repeated off-line signing of many transactions using the same signing key.

3. **Performing a multi-signer operation**: in this use case, a single transaction requires the signatures of many parties.  The unsigned transaction is first passed through one instance of solsign providing one key to use to sign the transaction.  The resulting partially signed transaction can be sent to the next signer, who will use solsign similarly to provide their signature.  This can be repeated numerous times until the transaction has been completely signed by all parties, at which point solsign will print out the completely signed transaction as well as its signature.

# A Note About Security #

**solsign is secure**.  It reads keypairs used for signing transactions, but does not output any sensitive information, including any aspect of private keys, nor does it write to any place other than the standard output.  And what it writes to standard output is completely secure, freely shareable information: encoded transactions as they would be sent over the network to solana validators for execution, plus the fee payer signature of those transactions, which is also used as the transaction id by the solana network.  Obviously transaction ids are not sensitive information as they are displayed and used all over the place.  To reiterate: **solsign never prints or transmits any sensitive information of any kind**.

After solsign has started up, the user can delete any key files used as input since solsign only reads them on start-up.  Please be careful, never delete the last copy of your keys!

The whole point of solsign is to sign transactions with maximum security -- locally on a user's computer, possibly with no connection to the internet whatsoever, and copying only transactions and signatures in and out.

solsign could be used with a very secure signing strategy such as copying Base64 encoded transactions to a USB drive, taking that USB drive to a completely airgapped computer, signing them there using solsign, and then copying the signed transactions back to the internet-connected computer that will submit them to the network using the same USB drive.  In this way, keys can live completely on an airgapped computer, but arbitrary transactions can still be signed and submitted to the network.

# Using solsign #

`solsign --help` will show brief help text describing its usage.

Pass the `--no-prompt` command line option to cause solsign to skip the "reading keys from standard input" step and to exit after a single transaction has been read in and processed.

All arguments besides `--help` and `--no-prompt` are paths to key files which will be read in and used to sign transactions.

After starting up, unless `--no-prompt` was specified, solsign will prompt for any additional keys that the user would like to supply for signing.  These are supplied as mnemonic seed phrases with optional passphrases, and after these values have been input, solsign will ask the user to select from one of several possible derivations of the key.

The key input sequence includes these steps:

1. solsign prints out the public keys of the current list of signing keys.
2. solsign prompts: `Enter mnemonic seed words of next key, or press ENTER to continue:`
3. If the user presses ENTER without supplying any other input, the key input phase is ended. **Note:** the mnemonic seed words will not be echoed to the screen as they are typed.
4. If the user enters a mnemomic word sequence, solsign then issues the prompt: `Enter passphrase seed, or press ENTER for no passphrase`
5. If the user pressed ENTER without entering a passphrase, then no passphrase is used.  **Note:** the passphrase will not be echoed to the screen as it is typed.
6. solsign then generates 9 possible derivations of the keypair.  The first is a direct seed using mnemonic and passphrase, as would be generated by the `solana-keygen` program.  The remaining are standard BIP-44 derivations as typically used by wallet software
7. The user selects the line that corresponds to their public key by entering the line number at the next propmpt: `Select a derived key 0 - 9 from above, or press ENTER to skip:`.  If the user presses ENTER without any number, then the derived keys are ignored.
8. This process repeats until the user presses ENTER at the mnemonic prompt indicating that there are no more keys to add.

After keys have been input, solscan prompts the user to enter a challenge password with this text:

`  Enter a password to be challenged with before each transaction is signed
  or press ENTER for no signing challenge password:`

The user may enter a password which they then must re-enter before any transaction will be signed by solsign.  **Note**
that the password will not be visible when typed at any point.  Entering a challenge password is **highly recommended**
as it protects the user from an intruder who could gain access to the command line and sign transactions using the
user's keys.  The intruder will not know the challenge password that was entered and thus will be unable to sign any
transactions.

solsign then enters a loop in which it reads Base64 encoded transactions from standard input, signs them, and prints results to standard output.  It stops after a single transaction if the `--no-prompt` command line argument was given, otherwise loops continuing to wait for and process transactions until standard input ends.

When a transaction is input, solsign signs it with whatever matching keys it has, and then if the transaction is completely signed, outputs:

`Transaction is complete:`

Followed by a Base64 encoded version of the completely signed transaction, and then followed by:

`Signature:`

With the signature of the transaction as the last item printed.

If the transaction was not completely signed, but still requires more signers, then solsign will print:

`Pubkeys still needed to sign:`

Followed by a list of public keys of the keypairs that still must sign the transaction, then followed by:

`Partially signed transaction:`

And then the Base64 encoded version of the partially signed transaction, ready to be used in subsequent solsign invocations to continue signing.

```
$ solsign --help

Usage: solsign [--help]
       solsign [--no-prompt] [KEY_FILE]...

  solsign reads Solana transactions in Base64 encoded format from stdin,
  displays them, signs them, writes signed transactions and signatures to
  stdout.

  On start-up, solsign reads any private key files specified on the command
  line. It also prompts for mnemonic and passcode combinations from stdin.
  Collectively these signing keys become available to the program to sign
  transactions.

  After reading in private keys, solsign enters a loop where it waits to read
  Base64 encoded transactions from standard input. After each encoded
  transaction is read in, for any signatures not provided within the
  transaction, if the key required for that signature was provided to solsign,
  the transaction will be signed with that key.

  After all possible signatures are applied, if the transaction is still not
  completely signed, then the list of pubkeys which must still sign the
  transaction is printed, along with the Base64 encoded version of the
  partially signed transaction is printed, ready for further signing.

  If after signing, the transaction is completely signed, then the signature
  of the transaction is printed.
```

# License #

Public domain, do what you want with it.
