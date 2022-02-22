import {ApiPromise} from '@polkadot/api';
import {Keyring} from '@polkadot/keyring';
import {
	provider,
	waitNfinalizedBlocks,
} from '../utils';
import {
	encodeFeeRecipientUpdateProposal,
	registerResourceId, 
	resourceId, 
	signAndSendUtil, 
	unsubSignedPropsUtil
} from './util/utils';
import {ethers} from 'ethers';
import {keccak256} from '@ethersproject/keccak256';
import {ECPair} from 'ecpair';
import {assert, u8aToHex} from '@polkadot/util';
import {feeRecipientUpdateProposal} from "./util/proposals";

async function testFeeRecipientUpdateProposal() {
	const api = await ApiPromise.create({provider});
	await registerResourceId(api);
	await sendFeeRecipientUpdateProposal(api);
	console.log('Waiting for the DKG to Sign the proposal');
	await waitNfinalizedBlocks(api, 8, 20 * 7);

	const dkgPubKeyCompressed: any = await api.query.dkg.dKGPublicKey();
	const dkgPubKey = ECPair.fromPublicKey(
		Buffer.from(dkgPubKeyCompressed[1].toHex().substr(2), 'hex'),
		{compressed: false}
	).publicKey.toString('hex');
	const chainIdType = api.createType('DkgRuntimePrimitivesChainIdType', {EVM: 5002});
	const propHash = keccak256(encodeFeeRecipientUpdateProposal(feeRecipientUpdateProposal));

	const proposalType = {feerecipientupdateproposal: feeRecipientUpdateProposal.header.nonce}

	const unsubSignedProps: any = await unsubSignedPropsUtil(api, chainIdType, dkgPubKey, proposalType, propHash);

	await new Promise((resolve) => setTimeout(resolve, 20000));

	unsubSignedProps();
}

async function sendFeeRecipientUpdateProposal(api: ApiPromise) {
	const keyring = new Keyring({type: 'sr25519'});
	const alice = keyring.addFromUri('//Alice');

	const [authoritySetId, dkgPubKey] = await Promise.all([
		api.query.dkg.authoritySetId(),
		api.query.dkg.dKGPublicKey(),
	]);

	const prop = u8aToHex(encodeFeeRecipientUpdateProposal(feeRecipientUpdateProposal));
	console.log(`DKG authority set id: ${authoritySetId}`);
	console.log(`DKG pub key: ${dkgPubKey}`);
	console.log(`Resource id is: ${resourceId}`);
	console.log(`Proposal is: ${prop}`);
	const chainIdType = api.createType('DkgRuntimePrimitivesChainIdType', {EVM: 5001});
	const kind = api.createType('DkgRuntimePrimitivesProposalProposalKind', 'FeeRecipientUpdate');
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
testFeeRecipientUpdateProposal()
	.catch(console.error)
	.finally(() => process.exit());