#!/usr/bin/env bash

set -e
ALICE=0xcb6df9de1efca7a3998a8ead4e02159d5fa99c3e0d4fd6432667390bb4726854
BOB=0x79c3b7fc0b7697b9414cb87adcb37317d1cab32818ae18c0e97ad76395d1fdcf
CHARLIE=0xf8d74108dbe199c4a6e4ef457046db37c325ba3f709b14cabfa1885663e4c589
echo "*** Start Webb DKG Node ***"
./target/release/dkg-standalone-node --tmp -lerror --node-key $ALICE &
./target/release/dkg-standalone-node --tmp -lerror --node-key $BOB &
./target/release/dkg-standalone-node --tmp \
    -lerror \
    -ldkg=debug \
    -ldkg_metadata=debug \
    -lruntime::offchain=debug \
    -ldkg_proposal_handler=debug \
    --node-key $CHARLIE
