/*
 * Copyright 2022 Webb Technologies Inc.
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 * http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 *
 */
import {ApiPromise} from '@polkadot/api';
import {Keyring} from '@polkadot/keyring';
import {
	ChainIdType,
	encodeSubstrateProposal, 
	makeResourceId,
	registerResourceId, 
	signAndSendUtil, 
	unsubSignedPropsUtil,
	substratePalletResourceId,
} from './util/utils';
import {
	provider,
	waitNfinalizedBlocks,
} from '../utils';
import {keccak256} from '@ethersproject/keccak256';
import {ECPair} from 'ecpair';
import {assert, u8aToHex} from '@polkadot/util';
import {getWrappingFeeUpdateProposal} from "./util/proposals";

async function testWrappingFeeUpdateProposal() {
	const api = await ApiPromise.create({provider});
	await registerResourceId(api);
	await sendWrappingFeeUpdateProposal(api);
	console.log('Waiting for the DKG to Sign the proposal');
	await waitNfinalizedBlocks(api, 8, 20 * 7);

	const dkgPubKeyCompressed: any = await api.query.dkg.dKGPublicKey();
	const dkgPubKey = ECPair.fromPublicKey(
		Buffer.from(dkgPubKeyCompressed[1].toHex().substr(2), 'hex'),
		{compressed: false}
	).publicKey.toString('hex');
	const chainIdType = api.createType('WebbProposalsHeaderTypedChainId', {SUBSTRATE: 5002});
	const propHash = keccak256(encodeSubstrateProposal(getWrappingFeeUpdateProposal(api), 3000));

	const proposalType = {wrappingfeeupdateproposal: getWrappingFeeUpdateProposal(api).header.nonce}

	const unsubSignedProps: any = await unsubSignedPropsUtil(api, chainIdType, dkgPubKey, proposalType, propHash);

	await new Promise((resolve) => setTimeout(resolve, 50000));

	unsubSignedProps();
}

async function sendWrappingFeeUpdateProposal(api: ApiPromise) {
	const keyring = new Keyring({type: 'sr25519'});
	const alice = keyring.addFromUri('//Alice');

	const [authoritySetId, dkgPubKey] = await Promise.all([
		api.query.dkg.authoritySetId(),
		api.query.dkg.dKGPublicKey(),
	]);

	const prop = u8aToHex(encodeSubstrateProposal(getWrappingFeeUpdateProposal(api), 3000));
	console.log(`DKG authority set id: ${authoritySetId}`);
	console.log(`DKG pub key: ${dkgPubKey}`);
	console.log(`Resource id is: ${substratePalletResourceId}`);
	console.log(`Proposal is: ${prop}`);
	const chainIdType = api.createType('WebbProposalsHeaderTypedChainId', {SUBSTRATE: 5001});
	const kind = api.createType('DkgRuntimePrimitivesProposalProposalKind', 'WrappingFeeUpdate');
	const proposal = api.createType('DkgRuntimePrimitivesProposal', {
		Unsigned: {
			kind: kind,
			data: prop
		}
	});
	const proposalCall = api.tx.dKGProposalHandler.forceSubmitUnsignedProposal(proposal);

	await signAndSendUtil(api, proposalCall, alice);
}

// Run
testWrappingFeeUpdateProposal()
	.catch(console.error)
	.finally(() => process.exit());
