use solana_client::client_error::ClientError;
use solana_program::instruction::InstructionError;
use solana_sdk::signature::Signature;
use std::fmt::Debug;
use tokio::task;

pub async fn retry<F, T, K, E, R>(arg: T, f: F, e: R) -> K
where
    F: Fn(&T) -> Result<K, E>,
    E: Debug,
    R: Fn(Result<K, E>) -> Result<K, E>,
{
    loop {
        let res = e(f(&arg));
        if res.is_ok() {
            return res.unwrap();
        }
        let error = res.err().unwrap();

        println!("Failed task with {:#?}, retrying", error);
        task::yield_now().await;
    }
}

pub fn no_op_filter(r: Result<Signature, ClientError>) -> Result<Signature, ClientError> {
    if let Err(e) = &r {
        match &e.kind {
            solana_client::client_error::ClientErrorKind::RpcError(
                solana_client::rpc_request::RpcError::RpcResponseError {
                    code: _,
                    message: _,
                    data,
                },
            ) => {
                if let solana_client::rpc_request::RpcResponseErrorData::SendTransactionPreflightFailure(f) = data {
                    match f.err {
                        Some(solana_sdk::transaction::TransactionError::InstructionError(_, InstructionError::Custom(0x5))) => {
                            println!("Operation was a no-op");
                            Ok(Signature::new(&[0;64]))
                        }
                        _ => r
                    }
                } else {
                    r
                }
            }
            _ => r,
        }
    } else {
        r
    }
}
