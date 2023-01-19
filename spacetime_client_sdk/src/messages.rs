use crate::client_api::{
    Message as ApiMessage, Message_oneof_type, TableRowOperation_OperationType,
};
use crate::web_socket::{ConnectionHandle, NetworkEvent};
use crate::ws::BuildConnection;
use protobuf::Message;
use serde::{Deserialize, Serialize};
use spacetimedb::spacetimedb_lib::{PrimaryKey, TupleDef, TupleValue};
use spacetimedb::TypeValue;
use tungstenite::Message as WsMessage;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SpaceDbResponse {
    FunctionCall(FunctionCallJson),
    SubscriptionUpdate(SubscriptionUpdateJson),
    Event(EventJson),
    TransactionUpdate(TransactionUpdateJson),
    IdentityToken(IdentityTokenJson),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityTokenJson {
    pub identity: String,
    pub token: String,
}

impl IdentityTokenJson {
    pub fn new(identity: &str, token: &str) -> Self {
        Self {
            identity: identity.to_string(),
            token: token.to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCallJson {
    pub reducer: String,
    pub arg_bytes: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableUpdateJson {
    pub table_id: u32,
    pub table_name: String,
    pub table_row_operations: Vec<TableRowOperationJson>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableRowOperationJson {
    pub op: TableOp,
    pub row_pk: String,
    pub row: Vec<TypeValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionUpdateJson {
    pub table_updates: Vec<TableUpdateJson>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventJson {
    pub timestamp: u64,
    pub status: String,          // committed, failed
    pub caller_identity: String, // hex identity
    pub function_call: FunctionCallJson,
    pub energy_quanta_used: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionUpdateJson {
    pub event: EventJson,
    pub subscription_update: SubscriptionUpdateJson,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StmtResultJson {
    pub schema: TupleDef,
    pub rows: Vec<Vec<TypeValue>>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Params {
    entity_id: u64,
    input: u8,
}
//pub fn create_new_player(identity: Hash, _timestamp: u64, entity_id: u64, input: u8) {

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct FnCall {
    #[serde(rename = "fn")]
    name: String,
    args: Vec<TypeValue>,
}

#[derive(Debug, Eq, PartialEq, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TableOp {
    Insert,
    Delete,
    Update,
}

#[derive(Debug)]
pub enum SpaceDbRequest {
    Ping,
    Pong,
    FunctionCall { name: String, args: Vec<TypeValue> },
}

fn make_args(of: Vec<TypeValue>) -> Vec<u8> {
    let mut bytes = Vec::new();

    let args = TupleValue {
        elements: of.into_boxed_slice(),
    };
    args.encode(&mut bytes);

    bytes
}

pub(crate) fn serialize_msg(
    con: &BuildConnection,
    msg: SpaceDbRequest,
) -> Option<tungstenite::Message> {
    match msg {
        SpaceDbRequest::FunctionCall { name, args } => {
            let call = FnCall { name, args };
            let json = serde_json::to_string(&call).unwrap();
            println!("{}", &json);
            Some(WsMessage::Text(json))
            // let args = make_args(args);
            //
            //let mut fun = FunctionCall::new();
            // fun.set_reducer(name);
            // fun.set_argBytes(args);
            // // let mut params = Vec::with_capacity(args.len() + 1);
            // // params.push(TypeValue::String(name.to_string()));
            // // params.extend_from_slice(&args);
            // let mut msg = ApiMessage::new();
            // msg.set_functionCall(fun);
            // let mut params = Vec::new();
            // msg.write_to_vec(&mut params).unwrap();
            // Some(WsMessage::Binary(params))
        }
        SpaceDbRequest::Ping | SpaceDbRequest::Pong => None,
    }
}

pub(crate) fn process_msg(
    msg: Result<tungstenite::Message, tungstenite::Error>,
) -> Option<NetworkEvent> {
    println!("Received MSG: {:?}", &msg);
    match msg {
        Ok(msg) => match msg {
            WsMessage::Text(txt) => {
                let handle = ConnectionHandle::new();
                let msg: SpaceDbResponse = serde_json::from_str(&txt).unwrap();
                Some(NetworkEvent::Message(handle, msg))
            }
            WsMessage::Binary(bin) => {
                let msg = ApiMessage::parse_from_bytes(&bin).unwrap();
                println!("Parsed: {:?}", &msg);

                if let Some(msg) = msg.field_type {
                    let handle = ConnectionHandle::new();

                    Some(NetworkEvent::Message(
                        handle,
                        match msg {
                            Message_oneof_type::identityToken(token) => {
                                //TODO: Fix &token.identity.to_string()
                                SpaceDbResponse::IdentityToken(IdentityTokenJson::new(
                                    "",
                                    &token.token,
                                ))
                            }
                            Message_oneof_type::subscriptionUpdate(ev) => {
                                let mut updates = Vec::with_capacity(ev.tableUpdates.len());

                                for x in ev.tableUpdates {
                                    let mut ops = Vec::with_capacity(x.tableRowOperations.len());

                                    for o in x.tableRowOperations {
                                        let op = match o.op {
                                            TableRowOperation_OperationType::DELETE => {
                                                TableOp::Delete
                                            }
                                            TableRowOperation_OperationType::INSERT => {
                                                TableOp::Insert
                                            }
                                        };

                                        dbg!(&o.row_pk);

                                        let (row_pk, len) = PrimaryKey::decode(&o.row_pk);
                                        // let row = TupleValue::decode()
                                        dbg!(&row_pk);
                                        // ops.push(TableRowOperationJson {
                                        //     op,
                                        //     row_pk: row_pk.to_string(),
                                        //     row: vec![],
                                        // })
                                    }

                                    let row = TableUpdateJson {
                                        table_id: x.tableId,
                                        table_name: x.tableName,
                                        table_row_operations: ops,
                                    };

                                    updates.push(row);
                                }

                                let up = SubscriptionUpdateJson {
                                    table_updates: updates,
                                };
                                SpaceDbResponse::SubscriptionUpdate(up)
                            }
                            Message_oneof_type::transactionUpdate(_) => return None,
                            Message_oneof_type::functionCall(_) | Message_oneof_type::event(_) => {
                                return None
                            }
                        },
                    ))
                } else {
                    return None;
                }
            }
            WsMessage::Ping(_) | WsMessage::Pong(_) | WsMessage::Frame(_) => return None,
            WsMessage::Close(_) => return None,
        },
        Err(err) => {
            eprintln!("{}", err);
            Some(NetworkEvent::Error(None, err.into()))
        }
    }
}
