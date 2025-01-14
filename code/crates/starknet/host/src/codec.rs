use bytes::Bytes;
use prost::Message;

use malachitebft_codec::Codec;
use malachitebft_core_types::{
    AggregatedSignature, CommitCertificate, CommitSignature, Extension, Round, SignedExtension,
    SignedProposal, SignedVote, Validity,
};
use malachitebft_engine::util::streaming::{StreamContent, StreamMessage};
use malachitebft_sync::{
    self as sync, ValueRequest, ValueResponse, VoteSetRequest, VoteSetResponse,
};

use malachitebft_core_consensus::{PeerId, ProposedValue, SignedConsensusMsg};
use malachitebft_starknet_p2p_proto::ConsensusMessage;

use crate::proto::consensus_message::Messages;
use crate::proto::{self as proto, Error as ProtoError, Protobuf};
use crate::types::{self as p2p, Address, BlockHash, Height, MockContext, ProposalPart, Vote};

trait MessageExt {
    fn encode_to_bytes(&self) -> Bytes;
}

impl<T> MessageExt for T
where
    T: Message,
{
    fn encode_to_bytes(&self) -> Bytes {
        Bytes::from(self.encode_to_vec())
    }
}

pub struct ProtobufCodec;

impl Codec<Address> for ProtobufCodec {
    type Error = ProtoError;

    fn decode(&self, bytes: Bytes) -> Result<Address, Self::Error> {
        Protobuf::from_bytes(&bytes)
    }

    fn encode(&self, address: &Address) -> Result<Bytes, Self::Error> {
        Protobuf::to_bytes(address)
    }
}

impl Codec<BlockHash> for ProtobufCodec {
    type Error = ProtoError;

    fn decode(&self, bytes: Bytes) -> Result<BlockHash, Self::Error> {
        Protobuf::from_bytes(&bytes)
    }

    fn encode(&self, block_hash: &BlockHash) -> Result<Bytes, Self::Error> {
        Protobuf::to_bytes(block_hash)
    }
}

impl Codec<ProposalPart> for ProtobufCodec {
    type Error = ProtoError;

    fn decode(&self, bytes: Bytes) -> Result<ProposalPart, Self::Error> {
        Protobuf::from_bytes(bytes.as_ref())
    }

    fn encode(&self, msg: &ProposalPart) -> Result<Bytes, Self::Error> {
        Protobuf::to_bytes(msg)
    }
}

pub fn decode_extension(ext: proto::Extension) -> Result<SignedExtension<MockContext>, ProtoError> {
    let extension = Extension::from(ext.data);
    let signature = ext
        .signature
        .ok_or_else(|| ProtoError::missing_field::<proto::Extension>("signature"))
        .and_then(p2p::Signature::from_proto)?;

    Ok(SignedExtension::new(extension, signature))
}

pub fn encode_extension(
    ext: &SignedExtension<MockContext>,
) -> Result<proto::Extension, ProtoError> {
    Ok(proto::Extension {
        data: ext.message.data.clone(),
        signature: Some(ext.signature.to_proto()?),
    })
}

impl Codec<SignedExtension<MockContext>> for ProtobufCodec {
    type Error = ProtoError;

    fn decode(&self, bytes: Bytes) -> Result<SignedExtension<MockContext>, Self::Error> {
        decode_extension(proto::Extension::decode(bytes)?)
    }

    fn encode(&self, msg: &SignedExtension<MockContext>) -> Result<Bytes, Self::Error> {
        encode_extension(msg).map(|proto| proto.encode_to_bytes())
    }
}

pub fn decode_proposed_value(
    proto: proto::sync::ProposedValue,
) -> Result<ProposedValue<MockContext>, ProtoError> {
    let proposer = proto
        .proposer
        .ok_or_else(|| ProtoError::missing_field::<proto::Proposal>("proposer"))?;

    Ok(ProposedValue {
        height: Height::new(proto.block_number, proto.fork_id),
        round: Round::from(proto.round),
        value: BlockHash::from_bytes(&proto.value)?,
        valid_round: Round::from(proto.valid_round),
        proposer: Address::from_proto(proposer)?,
        validity: Validity::from_bool(proto.validity),
        extension: proto.extension.map(decode_extension).transpose()?,
    })
}

pub fn encode_proposed_value(
    msg: &ProposedValue<MockContext>,
) -> Result<proto::sync::ProposedValue, ProtoError> {
    let proto = proto::sync::ProposedValue {
        fork_id: msg.height.fork_id,
        block_number: msg.height.block_number,
        round: msg.round.as_u32().expect("round should not be nil"),
        valid_round: msg.valid_round.as_u32(),
        value: msg.value.to_bytes()?,
        proposer: Some(msg.proposer.to_proto()?),
        validity: match msg.validity {
            Validity::Valid => true,
            Validity::Invalid => false,
        },
        extension: msg.extension.as_ref().map(encode_extension).transpose()?,
    };

    Ok(proto)
}

impl Codec<ProposedValue<MockContext>> for ProtobufCodec {
    type Error = ProtoError;

    fn decode(&self, bytes: Bytes) -> Result<ProposedValue<MockContext>, Self::Error> {
        decode_proposed_value(proto::sync::ProposedValue::decode(bytes)?)
    }

    fn encode(&self, msg: &ProposedValue<MockContext>) -> Result<Bytes, Self::Error> {
        encode_proposed_value(msg).map(|proto| proto.encode_to_bytes())
    }
}

pub fn decode_peer_id(proto: proto::PeerId) -> Result<PeerId, ProtoError> {
    PeerId::from_bytes(&proto.id).map_err(|e| ProtoError::Other(e.to_string()))
}

pub fn encode_peer_id(peer_id: &PeerId) -> Result<proto::PeerId, ProtoError> {
    Ok(proto::PeerId {
        id: Bytes::from(peer_id.to_bytes()),
    })
}

impl Codec<PeerId> for ProtobufCodec {
    type Error = ProtoError;

    fn decode(&self, bytes: Bytes) -> Result<PeerId, Self::Error> {
        decode_peer_id(proto::PeerId::decode(bytes)?)
    }

    fn encode(&self, peer_id: &PeerId) -> Result<Bytes, Self::Error> {
        encode_peer_id(peer_id).map(|proto| proto.encode_to_bytes())
    }
}

pub fn decode_sync_status(
    status: proto::sync::Status,
) -> Result<sync::Status<MockContext>, ProtoError> {
    let peer_id = status
        .peer_id
        .ok_or_else(|| ProtoError::missing_field::<proto::sync::Status>("peer_id"))?;

    Ok(sync::Status {
        peer_id: decode_peer_id(peer_id)?,
        height: Height::new(status.block_number, status.fork_id),
        history_min_height: Height::new(status.earliest_block_number, status.earliest_fork_id),
    })
}

pub fn encode_sync_status(
    status: &sync::Status<MockContext>,
) -> Result<proto::sync::Status, ProtoError> {
    Ok(proto::sync::Status {
        peer_id: Some(encode_peer_id(&status.peer_id)?),
        block_number: status.height.block_number,
        fork_id: status.height.fork_id,
        earliest_block_number: status.history_min_height.block_number,
        earliest_fork_id: status.history_min_height.fork_id,
    })
}

impl Codec<sync::Status<MockContext>> for ProtobufCodec {
    type Error = ProtoError;

    fn decode(&self, bytes: Bytes) -> Result<sync::Status<MockContext>, Self::Error> {
        decode_sync_status(proto::sync::Status::decode(bytes)?)
    }

    fn encode(&self, status: &sync::Status<MockContext>) -> Result<Bytes, Self::Error> {
        encode_sync_status(status).map(|proto| proto.encode_to_bytes())
    }
}

pub fn decode_sync_request(
    proto_request: proto::sync::SyncRequest,
) -> Result<sync::Request<MockContext>, ProtoError> {
    let messages = proto_request
        .messages
        .ok_or_else(|| ProtoError::missing_field::<proto::sync::SyncRequest>("messages"))?;
    let request = match messages {
        proto::sync::sync_request::Messages::ValueRequest(value_request) => {
            sync::Request::ValueRequest(ValueRequest::new(Height::new(
                value_request.block_number,
                value_request.fork_id,
            )))
        }
        proto::sync::sync_request::Messages::VoteSetRequest(vote_set_request) => {
            sync::Request::VoteSetRequest(VoteSetRequest::new(
                Height::new(vote_set_request.block_number, vote_set_request.fork_id),
                Round::new(vote_set_request.round),
            ))
        }
    };

    Ok(request)
}

pub fn encode_sync_request(
    request: &sync::Request<MockContext>,
) -> Result<proto::sync::SyncRequest, ProtoError> {
    let proto = match request {
        sync::Request::ValueRequest(value_request) => proto::sync::SyncRequest {
            messages: Some(proto::sync::sync_request::Messages::ValueRequest(
                proto::sync::ValueRequest {
                    fork_id: value_request.height.fork_id,
                    block_number: value_request.height.block_number,
                },
            )),
        },
        sync::Request::VoteSetRequest(vote_set_request) => proto::sync::SyncRequest {
            messages: Some(proto::sync::sync_request::Messages::VoteSetRequest(
                proto::sync::VoteSetRequest {
                    fork_id: vote_set_request.height.fork_id,
                    block_number: vote_set_request.height.block_number,
                    round: vote_set_request
                        .round
                        .as_u32()
                        .expect("round should not be nil"),
                },
            )),
        },
    };

    Ok(proto)
}

impl Codec<sync::Request<MockContext>> for ProtobufCodec {
    type Error = ProtoError;

    fn decode(&self, bytes: Bytes) -> Result<sync::Request<MockContext>, Self::Error> {
        decode_sync_request(proto::sync::SyncRequest::decode(bytes)?)
    }

    fn encode(&self, request: &sync::Request<MockContext>) -> Result<Bytes, Self::Error> {
        encode_sync_request(request).map(|proto| proto.encode_to_bytes())
    }
}

pub fn decode_sync_response(
    proto_response: proto::sync::SyncResponse,
) -> Result<sync::Response<MockContext>, ProtoError> {
    let messages = proto_response
        .messages
        .ok_or_else(|| ProtoError::missing_field::<proto::sync::SyncResponse>("messages"))?;

    let response = match messages {
        proto::sync::sync_response::Messages::ValueResponse(value_response) => {
            sync::Response::ValueResponse(ValueResponse::new(
                Height::new(value_response.block_number, value_response.fork_id),
                value_response.value.map(decode_synced_value).transpose()?,
            ))
        }
        proto::sync::sync_response::Messages::VoteSetResponse(vote_set_response) => {
            let height = Height::new(vote_set_response.block_number, vote_set_response.fork_id);
            let round = Round::new(vote_set_response.round);
            let vote_set = vote_set_response
                .vote_set
                .ok_or_else(|| ProtoError::missing_field::<proto::sync::VoteSet>("vote_set"))?;

            sync::Response::VoteSetResponse(VoteSetResponse::new(
                height,
                round,
                decode_vote_set(vote_set)?,
            ))
        }
    };
    Ok(response)
}

pub fn encode_sync_response(
    response: &sync::Response<MockContext>,
) -> Result<proto::sync::SyncResponse, ProtoError> {
    let proto = match response {
        sync::Response::ValueResponse(value_response) => proto::sync::SyncResponse {
            messages: Some(proto::sync::sync_response::Messages::ValueResponse(
                proto::sync::ValueResponse {
                    fork_id: value_response.height.fork_id,
                    block_number: value_response.height.block_number,
                    value: value_response
                        .value
                        .as_ref()
                        .map(encode_synced_value)
                        .transpose()?,
                },
            )),
        },
        sync::Response::VoteSetResponse(vote_set_response) => proto::sync::SyncResponse {
            messages: Some(proto::sync::sync_response::Messages::VoteSetResponse(
                proto::sync::VoteSetResponse {
                    fork_id: vote_set_response.height.fork_id,
                    block_number: vote_set_response.height.block_number,
                    round: vote_set_response
                        .round
                        .as_u32()
                        .expect("round should not be nil"),
                    vote_set: Some(encode_vote_set(&vote_set_response.vote_set)?),
                },
            )),
        },
    };

    Ok(proto)
}

impl Codec<sync::Response<MockContext>> for ProtobufCodec {
    type Error = ProtoError;

    fn decode(&self, bytes: Bytes) -> Result<sync::Response<MockContext>, Self::Error> {
        decode_sync_response(proto::sync::SyncResponse::decode(bytes)?)
    }

    fn encode(&self, response: &sync::Response<MockContext>) -> Result<Bytes, Self::Error> {
        encode_sync_response(response).map(|proto| proto.encode_to_bytes())
    }
}

pub fn decode_consensus_message(
    proto: proto::ConsensusMessage,
) -> Result<SignedConsensusMsg<MockContext>, ProtoError> {
    let proto_signature = proto
        .signature
        .ok_or_else(|| ProtoError::missing_field::<proto::ConsensusMessage>("signature"))?;

    let message = proto
        .messages
        .ok_or_else(|| ProtoError::missing_field::<proto::ConsensusMessage>("messages"))?;

    let signature = p2p::Signature::from_proto(proto_signature)?;

    match message {
        Messages::Vote(v) => {
            Vote::from_proto(v).map(|v| SignedConsensusMsg::Vote(SignedVote::new(v, signature)))
        }
        Messages::Proposal(p) => p2p::Proposal::from_proto(p)
            .map(|p| SignedConsensusMsg::Proposal(SignedProposal::new(p, signature))),
    }
}

pub fn encode_consensus_message(
    msg: &SignedConsensusMsg<MockContext>,
) -> Result<proto::ConsensusMessage, ProtoError> {
    let message = match msg {
        SignedConsensusMsg::Vote(v) => proto::ConsensusMessage {
            messages: Some(Messages::Vote(v.to_proto()?)),
            signature: Some(v.signature.to_proto()?),
        },
        SignedConsensusMsg::Proposal(p) => proto::ConsensusMessage {
            messages: Some(Messages::Proposal(p.to_proto()?)),
            signature: Some(p.signature.to_proto()?),
        },
    };

    Ok(message)
}

impl Codec<SignedConsensusMsg<MockContext>> for ProtobufCodec {
    type Error = ProtoError;

    fn decode(&self, bytes: Bytes) -> Result<SignedConsensusMsg<MockContext>, Self::Error> {
        decode_consensus_message(proto::ConsensusMessage::decode(bytes)?)
    }

    fn encode(&self, msg: &SignedConsensusMsg<MockContext>) -> Result<Bytes, Self::Error> {
        encode_consensus_message(msg).map(|proto| proto.encode_to_bytes())
    }
}

impl<T> Codec<StreamMessage<T>> for ProtobufCodec
where
    T: Protobuf,
{
    type Error = ProtoError;

    fn decode(&self, bytes: Bytes) -> Result<StreamMessage<T>, Self::Error> {
        let p2p_msg = p2p::StreamMessage::from_bytes(&bytes)?;

        Ok(StreamMessage {
            stream_id: p2p_msg.id,
            sequence: p2p_msg.sequence,
            content: match p2p_msg.content {
                p2p::StreamContent::Data(data) => {
                    StreamContent::Data(T::from_bytes(data.as_ref())?)
                }
                p2p::StreamContent::Fin(fin) => StreamContent::Fin(fin),
            },
        })
    }

    fn encode(&self, msg: &StreamMessage<T>) -> Result<Bytes, Self::Error> {
        let p2p_msg = p2p::StreamMessage {
            id: msg.stream_id,
            sequence: msg.sequence,
            content: match &msg.content {
                StreamContent::Data(data) => p2p::StreamContent::Data(data.to_bytes()?),
                StreamContent::Fin(fin) => p2p::StreamContent::Fin(*fin),
            },
        };

        p2p_msg.to_bytes()
    }
}

pub fn decode_aggregated_signature(
    signature: proto::sync::AggregatedSignature,
) -> Result<AggregatedSignature<MockContext>, ProtoError> {
    let signatures = signature
        .signatures
        .into_iter()
        .map(|s| {
            let signature = s
                .signature
                .ok_or_else(|| {
                    ProtoError::missing_field::<proto::sync::CommitSignature>("signature")
                })
                .and_then(p2p::Signature::from_proto)?;

            let address = s
                .validator_address
                .ok_or_else(|| {
                    ProtoError::missing_field::<proto::sync::CommitSignature>("validator_address")
                })
                .and_then(Address::from_proto)?;

            let extension = s.extension.map(decode_extension).transpose()?;

            Ok(CommitSignature {
                address,
                signature,
                extension,
            })
        })
        .collect::<Result<Vec<_>, ProtoError>>()?;

    Ok(AggregatedSignature { signatures })
}

pub fn encode_aggregate_signature(
    aggregated_signature: &AggregatedSignature<MockContext>,
) -> Result<proto::sync::AggregatedSignature, ProtoError> {
    let signatures = aggregated_signature
        .signatures
        .iter()
        .map(|s| {
            let validator_address = s.address.to_proto()?;
            let signature = s.signature.to_proto()?;

            Ok(proto::sync::CommitSignature {
                validator_address: Some(validator_address),
                signature: Some(signature),
                extension: s.extension.as_ref().map(encode_extension).transpose()?,
            })
        })
        .collect::<Result<_, ProtoError>>()?;

    Ok(proto::sync::AggregatedSignature { signatures })
}

impl Codec<AggregatedSignature<MockContext>> for ProtobufCodec {
    type Error = ProtoError;

    fn decode(&self, bytes: Bytes) -> Result<AggregatedSignature<MockContext>, Self::Error> {
        decode_aggregated_signature(proto::sync::AggregatedSignature::decode(bytes)?)
    }

    fn encode(&self, msg: &AggregatedSignature<MockContext>) -> Result<Bytes, Self::Error> {
        encode_aggregate_signature(msg).map(|proto| proto.encode_to_bytes())
    }
}

pub fn decode_certificate(
    certificate: proto::sync::CommitCertificate,
) -> Result<CommitCertificate<MockContext>, ProtoError> {
    let value_id = if let Some(block_hash) = certificate.block_hash {
        BlockHash::from_proto(block_hash)?
    } else {
        return Err(ProtoError::missing_field::<proto::sync::CommitCertificate>(
            "block_hash",
        ));
    };

    let aggregated_signature = if let Some(agg_sig) = certificate.aggregated_signature {
        decode_aggregated_signature(agg_sig)?
    } else {
        return Err(ProtoError::missing_field::<proto::sync::CommitCertificate>(
            "aggregated_signature",
        ));
    };

    let certificate = CommitCertificate {
        height: Height::new(certificate.block_number, certificate.fork_id),
        round: Round::new(certificate.round),
        value_id,
        aggregated_signature,
    };

    Ok(certificate)
}

pub fn encode_certificate(
    certificate: &CommitCertificate<MockContext>,
) -> Result<proto::sync::CommitCertificate, ProtoError> {
    Ok(proto::sync::CommitCertificate {
        fork_id: certificate.height.fork_id,
        block_number: certificate.height.block_number,
        round: certificate.round.as_u32().expect("round should not be nil"),
        block_hash: Some(certificate.value_id.to_proto()?),
        aggregated_signature: Some(encode_aggregate_signature(
            &certificate.aggregated_signature,
        )?),
    })
}

impl Codec<CommitCertificate<MockContext>> for ProtobufCodec {
    type Error = ProtoError;

    fn decode(&self, bytes: Bytes) -> Result<CommitCertificate<MockContext>, Self::Error> {
        decode_certificate(
            proto::sync::CommitCertificate::decode(bytes).map_err(ProtoError::Decode)?,
        )
    }

    fn encode(&self, msg: &CommitCertificate<MockContext>) -> Result<Bytes, Self::Error> {
        encode_certificate(msg).map(|proto| proto.encode_to_bytes())
    }
}

pub fn encode_synced_value(
    synced_value: &sync::RawDecidedValue<MockContext>,
) -> Result<proto::sync::SyncedValue, ProtoError> {
    Ok(proto::sync::SyncedValue {
        value_bytes: synced_value.value_bytes.clone(),
        certificate: Some(encode_certificate(&synced_value.certificate)?),
    })
}

pub fn decode_synced_value(
    proto: proto::sync::SyncedValue,
) -> Result<sync::RawDecidedValue<MockContext>, ProtoError> {
    let Some(certificate) = proto.certificate else {
        return Err(ProtoError::missing_field::<proto::sync::SyncedValue>(
            "certificate",
        ));
    };

    Ok(sync::RawDecidedValue {
        value_bytes: proto.value_bytes,
        certificate: decode_certificate(certificate)?,
    })
}

impl Codec<sync::RawDecidedValue<MockContext>> for ProtobufCodec {
    type Error = ProtoError;

    fn decode(&self, bytes: Bytes) -> Result<sync::RawDecidedValue<MockContext>, Self::Error> {
        let proto = proto::sync::SyncedValue::decode(bytes).map_err(ProtoError::Decode)?;
        decode_synced_value(proto)
    }

    fn encode(&self, msg: &sync::RawDecidedValue<MockContext>) -> Result<Bytes, Self::Error> {
        Ok(Bytes::from(encode_synced_value(msg)?.encode_to_vec()))
    }
}

pub(crate) fn encode_vote_set(
    vote_set: &malachitebft_core_types::VoteSet<MockContext>,
) -> Result<proto::sync::VoteSet, ProtoError> {
    Ok(proto::sync::VoteSet {
        signed_votes: vote_set
            .votes
            .iter()
            .map(encode_vote)
            .collect::<Result<Vec<_>, _>>()?,
    })
}

pub(crate) fn encode_vote(vote: &SignedVote<MockContext>) -> Result<ConsensusMessage, ProtoError> {
    Ok(ConsensusMessage {
        messages: Some(Messages::Vote(vote.message.to_proto()?)),
        signature: Some(vote.signature.to_proto()?),
    })
}

pub(crate) fn decode_vote_set(
    vote_set: proto::sync::VoteSet,
) -> Result<malachitebft_core_types::VoteSet<MockContext>, ProtoError> {
    Ok(malachitebft_core_types::VoteSet {
        votes: vote_set
            .signed_votes
            .into_iter()
            .filter_map(decode_vote)
            .collect(),
    })
}

pub(crate) fn decode_vote(msg: ConsensusMessage) -> Option<SignedVote<MockContext>> {
    let signature = msg.signature?;
    let vote = match msg.messages {
        Some(Messages::Vote(v)) => Some(v),
        _ => None,
    }?;

    let signature = p2p::Signature::from_proto(signature).ok()?;
    let vote = Vote::from_proto(vote).ok()?;
    Some(SignedVote::new(vote, signature))
}
