## solsign -- A utility for signing solana transactions off-line ##

# Introduction #

solsign is a small utility program that facilitates signing solana transactions off-line.  ***Off-line signing***
means using keys that are stored locally on the user's computer to sign transactions without connection to any
"wallet" software.

solsign reads complete solana transactions that have been encoded in Base64 format.  Base64 allows the binary data of
a solana transaction to be presented in only text characters suitable for copying and pasting into a terminal window.
solsign reads transactions, signs them, and then outputs the newly signed transaction (also in Base64 format), and
also outputs the signature of the transaction if the transaction is now completely signed.

solsign supports a few common workflows:

1. **Signing a single-signer transaction**: this is accomplished by passing the `--no-prompt` argument to solsign, along with the path to a key file.  solsign reads a transaction to sign from standard input, then signs it and outputs the signed version.

2. **Repeatedly signing single-signer transactions**: this is accomplished by passing the path to a key file to solsign on the command-line, **or** inputting the mnemonic and passphrase to solsign after it has started up.  After that, solsign will read transactions from standard input, sign them, and write the signatures to standard output, continuing to do so until the user ends the input.  This would be useful for repeated off-line signing of many transactions using the same signing key.

3. **Performing a multi-signer operation**: in this use case, a single transaction requires the signatures of many parties.  The unsigned transaction is first passed through one instance of solsign providing one key to use to sign the transaction.  The resulting partially signed transaction can be sent to the next signer, who will use solsign similarly to provide their signature.  This can be repeated numerous times until the transaction has been completely signed by all parties, at which point solsign will print out the completely signed transaction as well as its signature.

# A Note About Security #

**solsign is secure**.  It reads keypairs used for signing transactions, but does not output any sensitive information, including any aspect of private keys, nor does it write to any place other than the standard output.  And what it writes to standard output is completely secure, freely shareable information: encoded transactions as they would be sent over the network to solana validators for execution, plus the fee payer signature of those transactions, which is also used as the transaction id by the solana network.  Obviously transaction ids are not sensitive information as they are displayed and used all over the place.  To reiterate: **solsign never prints or transmits any sensitive information of any kind**.

After solsign has started up, the user can delete any key files used as input since solsign only reads them on start-up.  Please be careful, never delete the last copy of your keys!

The whole point of solsign is to sign transactions with maximum security -- locally on a user's computer, possibly with no connection to the internet whatsoever, and copying only transactions and signatures in and out.

solsign could be used with a very secure signing strategy such as copying Base64 encoded transactions to a USB drive, taking that USB drive to a completely airgapped computer, signing them there using solsign, and then copying the signed transactions back to the internet-connected computer that will submit them to the network using the same USB drive.  In this way, keys can live completely on an airgapped computer, but arbitrary transactions can still be signed and submitted to the network.

# Installing solsign #

You should build solsign from source.  To do so, first install the rust compiler if you have not done so already; instructions are here: https://www.rust-lang.org/tools/install

Next, clone the solsign source:

`git clone https://github.com/bji/solsign.git`

Finally, build it:

`cargo build --manifest-path solsign/Cargo.toml --release`

You will find the solsign binary at solsign/target/release/solsign.

**Prebuilt Binaries**

It is not recommended that you install a pre-built binary since you cannot be certain of the authenticity of the
binary.  However, if you're willing to take the chance, here is a Linux binary prebuilt by the author:

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

# Example Session #

Here is an example of using solsign being used to sign a transaction.  Comments are interspersed:

```
$ solsign /tmp/key.json
```

solsign is started with a single key file to use for signing (/tmp/key.json).
It is not necessary to start with a file; a user could instead just enter
the key using mnemonic and passphrase later.

```

  Public keys provided thus far:

    C7e6JVJQ2FnrLvETJ2KMHR8pvwSDJcQ2n8UTFaZz3yEy
```

solsign prints the keys that it knows about and can sign transactions with.

```

  Enter mnemonic seed words of next key, or press ENTER to continue:


  Enter passphrase seed, or press ENTER for no passphrase:
```

The user has chosen to enter a mnemonic seed phrase and passphrase.  These
are not displayed as they are typed.

```

  Derived Keys:

   (0)                      9xXVsPkh8jLwxQBJk6kf2CZp8ko27UHbiHCQifECLpbi
   (1)  m/44'/501'/0'/0'    CfRpvpSDq3kfvxrYyLqw1E1UQHnHD5aiLJ6vXeRJc98W
   (2)  m/44'/501'/0'/1'    4K1SvwTLrvYoyPGPpP5G1wiL72mgtJiJipEDewBAccD3
   (3)  m/44'/501'/0'/2'    4auo9rDdsaNuRTAvj3m5qGGRuRwMutk5XKACwbhrrx6K
   (4)  m/44'/501'/0'/3'    5FVg3XqMmFdPATZ2zeyMeoanp9eyVKfgBJxYYEq7PeDG
   (5)  m/44'/501'/0'/4'    ATBCmChQuf3jZDjB49F4sMaG5c7J38kioxKpsdc9MLuT
   (6)  m/44'/501'/0'/5'    BiVGWuLgQU8iCwQ7xWZi2tFApF7Vv51AxfhC5HPf8KQ5
   (7)  m/44'/501'/0'/6'    6BKweChix9m9Di9SnBFGvV6Py3XZBVgVf4M7DptDZJud
   (8)  m/44'/501'/0'/7'    qMxq1R1amcX5tmUk21bCt4WSqeD7mRqer74nuqYsBo8
   (9)  m/44'/501'/0'/8'    E29LTv4qHTevCjvZS3eT7qx8K5cKJJvbrKP2ufuz7a5D

  Select a derived key 0 - 9 from above, or press ENTER to skip: 1
  
```

The keys derived from the mnemonic and passphrase are shown and the user
selects (1) as their key.  The reason for deriving multiple keys is that
the same mnemonic and passphrase can generate keys differently depending
on the program that was used to generate the key.  The solana command line
would produce the key at (0), and most wallets would produce the key
at (1), unless the user has created multiple accounts from the same
wallet in which case some of the later key derivations may be the correct
one.

The user knows their public key (it's displayed by wallets, and it's not
sensitive information) which is how they knew which one to select.

```

  Public keys provided thus far:

    C7e6JVJQ2FnrLvETJ2KMHR8pvwSDJcQ2n8UTFaZz3yEy
    CfRpvpSDq3kfvxrYyLqw1E1UQHnHD5aiLJ6vXeRJc98W

  Enter mnemonic seed words of next key, or press ENTER to continue:

```

solsign shows that a new key is available for signing - the one that the
user just entered.  But the user presses ENTER now to decline entering
more keys.

```

  Enter a password to be challenged with before each transaction is signed
  or press ENTER for no signing challenge password:

```

The user has entered a password that will be used to challenge them
before every transaction is to be signed, to ensure that only the user
is able to sign transactions.  This helps in situations where solsign
is left running for a long time, waiting to accept new transactions
for signing.  The password challenge ensures that no one else can walk
up to the user's terminal and sign transactions.

Note that the challenge password is not saved anywhere and so for every
run of solsign the user can and should choose a new unique password
for that run.  If the password is forgotten, it's a simple thing to
restart solsign and type in a new password to use.


```

  Enter Base64 encoded transaction:

```

solsign is now waiting for transactions to be entered.

```

AQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABAAcPpSP6/XjGgrH6v7iCy7QQOeOQBPwglBAk9Jy6QyeSwSpPb+EUsfYZyL62JkOHBWY52pVBwq0oLK4uTwWtaNHTv3tz5xPtCaHjGkJYHJH/BA1HTzS7j/hQTPgHtfJEJzoHhRIKD3+birK4FGp9UCjyEDhlMU/LjvCCfMf2+XvsNWSOMvkiyxpjfBXOn304hEPQliTMS85DRtUEgx4Cxd9ergJlqycce6gJ0gKAD8LOUY6W3cIpDz7GqVwAc/9PSL6KoHEUh6vG6a8uC4MiZduBbIveTuvg6ssUHcgA9hS7utuumN1RTvJI/arQIHQgqWJ7SfBTaflWjdefWROfCmSOxQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAT0U9YrEr9wvxm7fiypSDWhnhQDstuaKolGxPYp1m5OqMlyWPTiSJ8bs9ECkUjg2DC1oTmdr/EIQEjnvY2+n4WZnNE8PV/r7jqaHrzOqvvKZNn5JYxJCHl/cO22HkjM+LC3BlsePRfEU4nVJ/awTDzVi4bHMaoP21SbbRvAP4KUYGlZAfwZF6jQzN8J60SCFssIJXW4MY1QnXxrDjna74MQbd9uHXZaGT2cvhRs7reawctIXtX1s3kTqM9YV+/wCpwD34W2dbMjp93Injd9sI5B+MqhHQPT6cjVRD78sNwNkBDRAACQQDBgECCwcABQ0OCgwIEAgAAAAAAAAAAGXNHQAAAAA=


  Enter challenge password (5 attempts remaining): 
  Enter challenge password (4 attempts remaining): 
  Enter challenge password (3 attempts remaining):

```

The user copy-pastes a Base64 encoded transaction in for signing, then takes
a few attempts to properly enter their challenge password.

```

  Transaction is complete:

    Ab+EThyFDLUilIm6Dm1VDcZ+6ivtOH5G77IrcdsCeFJ1avmPmO0NPDHzHB9FNQr4UYptrc+O
    6zi0r4sYg1s3KQABAAcPpSP6/XjGgrH6v7iCy7QQOeOQBPwglBAk9Jy6QyeSwSpPb+EUsfYZ
    yL62JkOHBWY52pVBwq0oLK4uTwWtaNHTv3tz5xPtCaHjGkJYHJH/BA1HTzS7j/hQTPgHtfJE
    JzoHhRIKD3+birK4FGp9UCjyEDhlMU/LjvCCfMf2+XvsNWSOMvkiyxpjfBXOn304hEPQliTM
    S85DRtUEgx4Cxd9ergJlqycce6gJ0gKAD8LOUY6W3cIpDz7GqVwAc/9PSL6KoHEUh6vG6a8u
    C4MiZduBbIveTuvg6ssUHcgA9hS7utuumN1RTvJI/arQIHQgqWJ7SfBTaflWjdefWROfCmSO
    xQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAT0U9YrEr9wvxm7fiypSDWhnhQDst
    uaKolGxPYp1m5OqMlyWPTiSJ8bs9ECkUjg2DC1oTmdr/EIQEjnvY2+n4WZnNE8PV/r7jqaHr
    zOqvvKZNn5JYxJCHl/cO22HkjM+LC3BlsePRfEU4nVJ/awTDzVi4bHMaoP21SbbRvAP4KUYG
    lZAfwZF6jQzN8J60SCFssIJXW4MY1QnXxrDjna74MQbd9uHXZaGT2cvhRs7reawctIXtX1s3
    kTqM9YV+/wCpwD34W2dbMjp93Injd9sI5B+MqhHQPT6cjVRD78sNwNkBDRAACQQDBgECCwcA
    BQ0OCgwIEAgAAAAAAAAAAGXNHQAAAAA=

  Signature:

   4q5sbcseTSPcc9V8iPDE6JMJtznbWDM9xaUWatPo8sk8c65RQxobuDFWvwxFKFQPh7b2yDvbHh1YXVVgYKdQwrMu

```

The keys that the user supplied to solsign were sufficient for signing the
transaction.  The completely signed transaction is printed out in Base64
encoding, as well as the transaction signature (i.e. transaction id) of
the transaction.

Hopefully the user has a program available to them for accepting this text
and submitting the signed transaction for execution.  The author, for
example, has a Discord bot that will do this as part of a defi system
under development.

```

  Enter Base64 encoded transaction:

$
```

solsign waits for another transaction to sign, but the user instead
typed Ctrl-D to end input; solsign exits and the user is returned to
their shell prompt.

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
