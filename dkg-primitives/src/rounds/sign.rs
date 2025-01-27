// Copyright 2022 Webb Technologies Inc.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//
use codec::Encode;
use curv::arithmetic::Converter;
use log::{debug, error, trace, warn};
use sp_core::ecdsa::Signature;
use sp_runtime::traits::AtLeast32BitUnsigned;
use std::collections::BTreeMap;

use crate::{types::*, utils::vec_usize_to_u16};

pub use gg_2020::{
	party_i::*,
	state_machine::{keygen::*, sign::*},
};
pub use multi_party_ecdsa::protocols::multi_party_ecdsa::{
	gg_2020,
	gg_2020::state_machine::{keygen as gg20_keygen, sign as gg20_sign, traits::RoundBlame},
};

/// Wrapper state-machine for Sign rounds
pub enum SignState<Clock>
where
	Clock: AtLeast32BitUnsigned + Copy,
{
	NotStarted(PreSignRounds),
	Started(SignRounds<Clock>),
	Finished(Result<DKGSignedPayload, DKGError>),
}

/// Implementation of DKGRoundsSM trait that dispatches calls
/// to the current internal state if applicable.
impl<C> DKGRoundsSM<DKGVoteMessage, SignState<C>, C> for SignState<C>
where
	C: AtLeast32BitUnsigned + Copy,
{
	fn proceed(&mut self, at: C) -> Result<bool, DKGError> {
		match self {
			Self::Started(sign_rounds) => sign_rounds.proceed(at),
			_ => Ok(true),
		}
	}

	fn get_outgoing(&mut self) -> Vec<DKGVoteMessage> {
		match self {
			Self::Started(sign_rounds) => sign_rounds.get_outgoing(),
			_ => Vec::new(),
		}
	}

	fn handle_incoming(&mut self, data: DKGVoteMessage, at: C) -> Result<(), DKGError> {
		match self {
			Self::NotStarted(pre_rounds) => pre_rounds.handle_incoming(data, at),
			Self::Started(sign_rounds) => sign_rounds.handle_incoming(data, at),
			_ => Ok(()),
		}
	}

	fn is_finished(&self) -> bool {
		match self {
			Self::Started(sign_rounds) => sign_rounds.is_finished(),
			_ => true,
		}
	}

	fn try_finish(self) -> Result<Self, DKGError> {
		match self {
			Self::Started(ref sign_rounds) =>
				if sign_rounds.is_finished() {
					Ok(self)
				} else {
					Err(DKGError::SMNotFinished)
				},
			_ => Ok(self),
		}
	}
}

/// Pre-sign rounds
/// Used to collect incoming messages from other peers which started earlier
pub struct PreSignRounds {
	pub pending_sign_msgs: Vec<DKGVoteMessage>,
}

impl PreSignRounds {
	pub fn new() -> Self {
		Self { pending_sign_msgs: Vec::default() }
	}
}

impl Default for PreSignRounds {
	fn default() -> Self {
		Self::new()
	}
}

impl<C> DKGRoundsSM<DKGVoteMessage, Vec<DKGVoteMessage>, C> for PreSignRounds
where
	C: AtLeast32BitUnsigned + Copy,
{
	fn handle_incoming(&mut self, data: DKGVoteMessage, _at: C) -> Result<(), DKGError> {
		self.pending_sign_msgs.push(data);
		Ok(())
	}

	fn is_finished(&self) -> bool {
		true
	}

	fn try_finish(self) -> Result<Vec<DKGVoteMessage>, DKGError> {
		Ok(self.pending_sign_msgs)
	}
}

/// Sign rounds
/// Main state, corresponds to gg20 SignManual one round signing object
pub struct SignRounds<Clock>
where
	Clock: AtLeast32BitUnsigned + Copy,
{
	params: SignParams,
	round_key: Vec<u8>,
	sign_tracker: DKGRoundTracker<Vec<u8>, Clock>,
	sign_outgoing_msgs: Vec<DKGVoteMessage>,
}

impl<C> SignRounds<C>
where
	C: AtLeast32BitUnsigned + Copy,
{
	pub fn new(
		params: SignParams,
		started_at: C,
		payload: Vec<u8>,
		round_key: Vec<u8>,
		partial_sig: PartialSignature,
		sign_manual: SignManual,
	) -> Self {
		let sign_tracker = DKGRoundTracker::<std::vec::Vec<u8>, C> {
			sign_manual: Some(sign_manual),
			payload: Some(payload),
			started_at,
			..Default::default()
		};

		let mut sign_outgoing_msgs: Vec<DKGVoteMessage> = Vec::new();
		let serialized = serde_json::to_string(&partial_sig).unwrap();
		let msg = DKGVoteMessage {
			party_ind: params.party_index,
			round_key: round_key.clone(),
			partial_signature: serialized.into_bytes(),
		};
		sign_outgoing_msgs.push(msg);

		Self { params, round_key, sign_tracker, sign_outgoing_msgs }
	}
}

impl<C> DKGRoundsSM<DKGVoteMessage, DKGSignedPayload, C> for SignRounds<C>
where
	C: AtLeast32BitUnsigned + Copy,
{
	/// Proceed to next step

	fn proceed(&mut self, at: C) -> Result<bool, DKGError> {
		if self.sign_tracker.is_done(self.params.threshold) {
			return Ok(true)
		}
		// Check if we have signed the message and that the timeout has exhausted
		let signed_by_self = self.sign_tracker.is_signed_by(self.params.party_index);
		let sign_timeout_exhausted = at - self.sign_tracker.started_at > SIGN_TIMEOUT.into();
		// If so, identify all bad actors and return an error
		if signed_by_self && sign_timeout_exhausted {
			let signed_by = self.sign_tracker.get_signed_parties();
			let mut not_signed_by: Vec<u16> = self
				.params
				.signers
				.iter()
				.filter(|v| !signed_by.contains(*v))
				.copied()
				.collect();

			let mut bad_actors: Vec<u16> = Vec::new();
			bad_actors.append(&mut not_signed_by);

			Err(DKGError::SignTimeout { bad_actors })
		} else {
			Ok(false)
		}
	}

	/// Get outgoing messages

	fn get_outgoing(&mut self) -> Vec<DKGVoteMessage> {
		trace!(target: "dkg", "🕸️  Getting outgoing vote messages");
		std::mem::take(&mut self.sign_outgoing_msgs)
	}

	/// Handle incoming messages

	fn handle_incoming(&mut self, data: DKGVoteMessage, _at: C) -> Result<(), DKGError> {
		trace!(target: "dkg", "🕸️  Handle vote message");

		if data.party_ind == self.params.party_index {
			warn!(target: "dkg", "🕸️  Ignore messages sent by self");
			return Ok(())
		}

		let sig: PartialSignature = match serde_json::from_slice(&data.partial_signature) {
			Ok(sig) => sig,
			Err(err) => {
				error!(target: "dkg", "🕸️  Error deserializing msg: {:?}", err);
				return Err(DKGError::GenericError {
					reason: "Error deserializing vote msg".to_string(),
				})
			},
		};

		self.sign_tracker.add_vote(data.party_ind, sig);

		Ok(())
	}

	/// Try finish

	fn is_finished(&self) -> bool {
		self.sign_tracker.is_done(self.params.threshold)
	}

	fn try_finish(self) -> Result<DKGSignedPayload, DKGError> {
		let payload = self.sign_tracker.payload.clone();
		let sig = self.sign_tracker.complete();

		if let Err(err) = sig {
			println!("{:?}", err);
			Err(err)
		} else if let (Some(payload), Ok(sig)) = (payload, sig) {
			match convert_signature(&sig) {
				Some(signature) => {
					let signed_payload = DKGSignedPayload {
						key: self.round_key.clone(),
						payload,
						signature: signature.encode(),
					};

					debug!(target: "dkg", "🕸️  Finished round w/ key: {:?}", self.round_key);
					Ok(signed_payload)
				},
				_ => Err(DKGError::GenericError {
					reason: "Error serializing signature".to_string(),
				}),
			}
		} else {
			Err(DKGError::GenericError { reason: "No payload".to_string() })
		}
	}
}

/// Convenience struct to accumulate partial signatures
/// and later construct the full signature.
struct DKGRoundTracker<Payload, Clock> {
	votes: BTreeMap<u16, PartialSignature>,
	sign_manual: Option<SignManual>,
	payload: Option<Payload>,
	started_at: Clock,
}

impl<P, C> Default for DKGRoundTracker<P, C>
where
	C: AtLeast32BitUnsigned + Copy,
{
	fn default() -> Self {
		Self {
			votes: Default::default(),
			sign_manual: Default::default(),
			payload: Default::default(),
			started_at: 0u32.into(),
		}
	}
}

impl<P, C> DKGRoundTracker<P, C>
where
	C: AtLeast32BitUnsigned + Copy,
{
	fn add_vote(&mut self, party: u16, vote: PartialSignature) -> bool {
		self.votes.insert(party, vote);
		true
	}

	fn is_signed_by(&self, party: u16) -> bool {
		self.votes.contains_key(&party)
	}

	fn get_signed_parties(&self) -> Vec<u16> {
		self.votes.keys().copied().collect()
	}

	fn is_done(&self, threshold: u16) -> bool {
		self.sign_manual.is_some() && self.votes.len() >= threshold as usize
	}

	fn complete(mut self) -> Result<SignatureRecid, DKGError> {
		if let Some(sign_manual) = self.sign_manual.take() {
			debug!(target: "dkg", "Trying to complete vote with {} votes", self.votes.len());

			let votes: Vec<PartialSignature> = self.votes.into_values().collect();

			return match sign_manual.complete(&votes) {
				Ok(sig) => {
					debug!("Obtained complete signature: {}", &sig.recid);
					Ok(sig)
				},
				Err(err) => {
					let sign_err = match err {
						SignError::LocalSigning(sign_err) => sign_err,
						SignError::CompleteSigning(sign_err) => sign_err,
					};

					match sign_err {
						gg20_sign::rounds::Error::Round1(err_type) |
						gg20_sign::rounds::Error::Round2Stage4(err_type) |
						gg20_sign::rounds::Error::Round3(err_type) |
						gg20_sign::rounds::Error::Round5(err_type) |
						gg20_sign::rounds::Error::Round6VerifyProof(err_type) =>
							return Err(DKGError::SignMisbehaviour {
								bad_actors: vec_usize_to_u16(err_type.bad_actors),
							}),
						_ => return Err(DKGError::GenericError { reason: sign_err.to_string() }),
					}
				},
			}
		}
		Err(DKGError::GenericError { reason: "No SignManual found".to_string() })
	}
}

pub fn convert_signature(sig_recid: &SignatureRecid) -> Option<Signature> {
	let r = sig_recid.r.to_bigint().to_bytes();
	let s = sig_recid.s.to_bigint().to_bytes();
	let v = sig_recid.recid + 27u8;

	let mut sig_vec: Vec<u8> = Vec::new();

	for _ in 0..(32 - r.len()) {
		sig_vec.extend(&[0]);
	}
	sig_vec.extend_from_slice(&r);

	for _ in 0..(32 - s.len()) {
		sig_vec.extend(&[0]);
	}
	sig_vec.extend_from_slice(&s);

	sig_vec.extend(&[v]);

	if 65 != sig_vec.len() {
		warn!(target: "dkg", "🕸️  Invalid signature len: {}, expected 65", sig_vec.len());
		return None
	}

	let mut dkg_sig_arr: [u8; 65] = [0; 65];
	dkg_sig_arr.copy_from_slice(&sig_vec[0..65]);

	Some(Signature(dkg_sig_arr))
}
