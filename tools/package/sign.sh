#!/bin/sh
# Signs release files with the project's GPG key, or checks the signatures they
# arrived with.
#
#   tools/package/sign.sh sign  <file>...
#   tools/package/sign.sh verify <file>...
#
# The key comes from the environment, never from the repository:
#
#   GPG_PRIVATE_KEY   the ASCII-armoured private key (a GitHub secret)
#   GPG_PASSPHRASE    its passphrase, if it has one
#
# Without a key, `sign` says so and exits non-zero instead of producing
# unsigned-looking-signed output. `verify` needs only gpg.
set -eu

mode=${1:?usage: sign.sh sign|verify <file>...}
shift
[ "$#" -gt 0 ] || { echo "sign.sh: no files given" >&2; exit 2; }

case "$mode" in
sign)
    [ -n "${GPG_PRIVATE_KEY:-}" ] || {
        echo "sign.sh: GPG_PRIVATE_KEY is not set, so nothing was signed" >&2
        exit 1
    }
    # A throwaway home: the runner has no keyring, and importing into the user's
    # would make the signature depend on whatever else is in it.
    home=$(mktemp -d)
    trap 'rm -rf "$home"' EXIT
    export GNUPGHOME="$home"
    chmod 700 "$home"

    printf '%s\n' "$GPG_PRIVATE_KEY" | gpg --batch --import 2>/dev/null
    key=$(gpg --list-secret-keys --with-colons | awk -F: '/^fpr:/ {print $10; exit}')
    [ -n "$key" ] || { echo "sign.sh: the imported key has no fingerprint" >&2; exit 1; }

    if [ -n "${GPG_PASSPHRASE:-}" ]; then
        # Loopback with a passphrase on stdin, so nothing is echo'd into a log.
        sign() {
            printf '%s\n' "$GPG_PASSPHRASE" |
                gpg --batch --yes --pinentry-mode loopback --passphrase-fd 0 \
                    --local-user "$key" --armor --detach-sign "$1"
        }
    else
        sign() {
            gpg --batch --yes --pinentry-mode loopback --passphrase '' \
                --local-user "$key" --armor --detach-sign "$1"
        }
    fi

    for file in "$@"; do
        sign "$file"
        gpg --armor --export "$key" > "$(dirname "$file")/rustmagic-signing-key.asc"
        echo "signed $file"
    done
    ;;
verify)
    for file in "$@"; do
        [ -f "$file" ] || { echo "sign.sh: $file does not exist" >&2; exit 1; }
        gpg --verify "$file.asc" "$file"
    done
    ;;
*)
    echo "sign.sh: unknown mode $mode (sign or verify)" >&2
    exit 2
    ;;
esac
