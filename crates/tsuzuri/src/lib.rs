//! Common data types for CQRS and Event Sourcing.

// procedural macro で使用するために、現在のクレートを `tsuzuri` という名前で再エクスポートします。
pub use crate as tsuzuri;

#[macro_use]
mod macros;

pub use tsuzuri_derive::*;

mod error;

pub mod aggregate;
pub mod store;

use crate::{
    aggregate::{Aggregate, Apply, Handle, State},
    store::{
        payload::Payload,
        sync::{
            error::StoreError, event_store::EventStore, query_store::QueryStore, reader::ReadStore, writer::WriteStore,
        },
    },
};
use std::{collections::HashMap, sync::Arc};

pub struct Tsuzuri<Q> {
    event_store: Arc<EventStore>,
    query_store: Q,
}

pub struct TsuzuriBuilder<Q> {
    event_store: EventStore,
    query_store: Q,
}

impl TsuzuriBuilder<NoQueryStore> {
    pub fn new(event_store: EventStore) -> Self {
        Self {
            event_store,
            query_store: NoQueryStore,
        }
    }
}

impl<Q> TsuzuriBuilder<Q> {
    /// reader を設定すると状態が WithReader に変わる
    pub fn query_store(self, query: QueryStore) -> TsuzuriBuilder<WithQueryStore> {
        TsuzuriBuilder {
            event_store: self.event_store,
            query_store: WithQueryStore(Arc::new(query)),
        }
    }

    /// 現在の状態で Tsuzuri を生成する
    pub fn build(self) -> Tsuzuri<Q> {
        Tsuzuri {
            event_store: Arc::new(self.event_store),
            query_store: self.query_store,
        }
    }
}

/// reader 未設定を表すマーカー型
pub struct NoQueryStore;
/// reader 設定済みをラップする型
pub struct WithQueryStore(Arc<QueryStore>);

impl Tsuzuri<WithQueryStore> {
    pub fn qs_write(&self) -> WriteStore {
        self.query_store.0.write_store.clone()
    }

    pub fn qs_read(&self) -> ReadStore {
        self.query_store.0.read_store.clone()
    }
}

impl<Q> Tsuzuri<Q> {
    pub fn new(event_store: EventStore, query_store: Q) -> Self {
        Self {
            event_store: event_store.into(),
            query_store,
        }
    }

    pub fn es_write(&self) -> WriteStore {
        self.event_store.write_store.clone()
    }

    pub fn es_read(&self) -> ReadStore {
        self.event_store.read_store.clone()
    }

    pub async fn execute<T>(&self, id: &str, cmd: T::Command) -> Result<(), StoreError>
    where
        T: Aggregate,
        State<T>: Apply<T::Event> + Handle<T::Command>,
    {
        self.execute_with_metadata(id, cmd, HashMap::new()).await
    }

    pub async fn execute_with_metadata<T>(
        &self,
        id: &str,
        cmd: T::Command,
        metadata: HashMap<String, String>,
    ) -> Result<(), StoreError>
    where
        T: Aggregate,
        State<T>: Apply<T::Event> + Handle<T::Command>,
    {
        // 集約を再生する
        let mut current_sequence = 0;
        let events = self.es_read().read_to_latest(id, current_sequence).await?;
        let mut agg = State::<T>::init(id.to_string());
        for envelope in events {
            current_sequence = envelope.sequence;
            let event = serde_json::from_slice::<T::Event>(&envelope.bytes).unwrap();
            agg.apply(event);
        }
        // 再生した集約にコマンドを適用する
        let events = agg.handle(cmd).unwrap();
        // イベントを書き込む
        for event in events {
            current_sequence += 1;
            let payload = Payload::new(
                id,
                current_sequence,
                serde_json::to_vec(&event).unwrap(),
                Some(serde_json::to_vec(&metadata).unwrap()),
            )
            .unwrap();
            self.es_write().write(id, payload).await?;
        }
        // クエリを同期的に更新する
        Ok(())
    }
}

#[doc(hidden)]
pub mod __macro_helpers {
    use serde_json::Value;
    pub use {serde_json, tracing, tracing_tunnel};

    /// Extracts the event name and payload from an event json value.
    /// `{"EventName": {"foo": 1}}` returns `("EventName", {"foo": 1})`.
    pub fn extract_event_name_payload(value: Value) -> Result<(String, Value), &'static str> {
        let Value::Object(map) = value else {
            return Err("event is not an object");
        };

        let mut iter = map.into_iter();
        let Some((event, payload)) = iter.next() else {
            return Err("event is empty");
        };

        if iter.next().is_some() {
            return Err("event contains multiple keys");
        }

        Ok((event, payload))
    }
}

mod tests {
    #[allow(unused_imports)]
    use super::*;
    use crate::{
        aggregate::{Aggregate, Apply, Handle},
        events, Command, Event,
    };
    #[allow(unused_imports)]
    use serde::{Deserialize, Serialize};
    use thiserror::Error;

    #[derive(Debug, Error, Serialize)]
    pub enum BankAccountError {
        #[error("account already opened")]
        AccountAlreadyOpened,
        #[error("account not open")]
        AccountNotOpen,
        #[error("cannot withdraw/deposit an amount of 0")]
        AmountIsZero,
        #[error("insufficient balance")]
        InsufficientBalance,
    }

    #[derive(Debug, Default)]
    pub struct BankAccount {
        opened: bool,
        balance: i64,
    }

    impl Aggregate for BankAccount {
        type Command = BankAccountCommand;
        type Event = BankAccountEvent;

        fn init(_id: String) -> Self {
            BankAccount {
                opened: false,
                balance: 0,
            }
        }
    }

    #[derive(Deserialize, Command)]
    pub enum BankAccountCommand {
        OpenAccount(OpenAccount),
        DepositFunds(DepositFunds),
        WithdrawFunds(WithdrawFunds),
    }
    #[derive(Deserialize)]
    pub struct OpenAccount {}

    impl Handle<OpenAccount> for BankAccount {
        type Error = BankAccountError;

        fn handle(&self, _cmd: OpenAccount) -> Result<Vec<BankAccountEvent>, Self::Error> {
            if self.opened {
                return Err(BankAccountError::AccountAlreadyOpened);
            }

            events![AccountOpened {}]
        }
    }

    #[derive(Deserialize)]
    pub struct DepositFunds {
        amount: u32,
    }

    impl Handle<DepositFunds> for BankAccount {
        type Error = BankAccountError;

        fn handle(&self, cmd: DepositFunds) -> Result<Vec<BankAccountEvent>, Self::Error> {
            if !self.opened {
                return Err(BankAccountError::AccountNotOpen);
            }

            if cmd.amount == 0 {
                return Err(BankAccountError::AmountIsZero);
            }

            events![FundsDeposited { amount: cmd.amount }]
        }
    }

    #[derive(Deserialize)]
    pub struct WithdrawFunds {
        amount: u32,
    }

    impl Handle<WithdrawFunds> for BankAccount {
        type Error = BankAccountError;

        fn handle(&self, cmd: WithdrawFunds) -> Result<Vec<BankAccountEvent>, Self::Error> {
            if !self.opened {
                return Err(BankAccountError::AccountNotOpen);
            }

            if cmd.amount == 0 {
                return Err(BankAccountError::AmountIsZero);
            }

            let new_balance = self.balance - cmd.amount as i64;
            if new_balance < 0 {
                return Err(BankAccountError::InsufficientBalance);
            }

            events![FundsWithdrawn { amount: cmd.amount }]
        }
    }

    #[derive(Clone, Debug, Event, Serialize, Deserialize)]
    pub enum BankAccountEvent {
        OpenedAccount(AccountOpened),
        DepositedFunds(FundsDeposited),
        WithdrewFunds(FundsWithdrawn),
    }

    #[derive(Clone, Debug, Default, Serialize, Deserialize)]
    pub struct AccountOpened {}

    impl Default for BankAccountEvent {
        fn default() -> Self {
            BankAccountEvent::OpenedAccount(AccountOpened::default())
        }
    }

    impl Apply<AccountOpened> for BankAccount {
        fn apply(&mut self, _event: AccountOpened) {
            self.opened = true;
        }
    }

    #[derive(Clone, Debug, Default, Serialize, Deserialize)]
    pub struct FundsDeposited {
        pub amount: u32,
    }

    impl Apply<FundsDeposited> for BankAccount {
        fn apply(&mut self, event: FundsDeposited) {
            self.balance += event.amount as i64;
        }
    }

    #[derive(Clone, Debug, Default, Serialize, Deserialize)]
    pub struct FundsWithdrawn {
        pub amount: u32,
    }

    impl Apply<FundsWithdrawn> for BankAccount {
        fn apply(&mut self, event: FundsWithdrawn) {
            self.balance -= event.amount as i64;
        }
    }

    #[tokio::test]
    async fn test_write_with_writer() -> Result<(), StoreError> {
        use crate::store::sync::memory_store::MemoryStore;

        let query_store = QueryStore::new(MemoryStore::new());
        let event_store = EventStore::new(MemoryStore::new());

        // event_store: 必須
        // query_store: 任意
        // Axumでは、AppStateに格納することで、アプリケーション全体で共有することができる。
        let tsuzuri = TsuzuriBuilder::new(event_store).query_store(query_store).build();

        let id = "test_1_A";

        // それぞれのコマンド実行する時に以下を呼び出す。
        let cmd = BankAccountCommand::OpenAccount(OpenAccount {});
        tsuzuri.execute::<BankAccount>(id, cmd).await?;

        let cmd = BankAccountCommand::DepositFunds(DepositFunds { amount: 100 });
        tsuzuri.execute::<BankAccount>(id, cmd).await?;

        let cmd = BankAccountCommand::DepositFunds(DepositFunds { amount: 200 });
        tsuzuri.execute::<BankAccount>(id, cmd).await?;

        let cmd = BankAccountCommand::WithdrawFunds(WithdrawFunds { amount: 50 });
        tsuzuri.execute::<BankAccount>(id, cmd).await?;

        assert_eq!(tsuzuri.es_read().read_to_latest(id, 0).await?.len(), 4);

        Ok(())
    }
}
