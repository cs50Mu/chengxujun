use axum::{
    extract::{ws::Message, WebSocketUpgrade},
    response::IntoResponse,
    Extension,
};
use dashmap::{DashMap, DashSet};
use futures::{Sink, SinkExt, Stream, StreamExt};
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::log::warn;
pub use ws_shared::{Msg, MsgData};

#[derive(Debug)]
struct State {
    // for a given user, how many rooms they are in
    user_rooms: DashMap<String, DashSet<String>>,
    // for a given room, how many users are in it
    room_users: DashMap<String, DashSet<String>>,
    tx: broadcast::Sender<Arc<Msg>>,
}

#[derive(Debug, Clone)]
pub struct ChatState(Arc<State>);

impl State {
    fn new() -> Self {
        let (tx, _rx) = broadcast::channel(20);
        Self {
            user_rooms: Default::default(),
            room_users: Default::default(),
            tx,
        }
    }
}

impl ChatState {
    pub fn new() -> Self {
        Self(Arc::new(State::new()))
    }

    pub fn get_user_rooms(&self, username: &str) -> Vec<String> {
        self.0
            .user_rooms
            .get(username)
            .map(|rooms| rooms.clone().into_iter().collect())
            .unwrap_or_default()
    }

    pub fn get_room_users(&self, room: &str) -> Vec<String> {
        self.0
            .room_users
            .get(room)
            .map(|users| users.clone().into_iter().collect())
            .unwrap_or_default()
    }
}

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Extension(state): Extension<ChatState>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket<S>(socket: S, state: ChatState)
where
    S: Stream<Item = Result<Message, axum::Error>> + Sink<Message> + Send + 'static,
{
    let mut rx = state.0.tx.subscribe();
    let (mut sender, mut receiver) = socket.split();

    let state1 = state.clone();
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(data)) = receiver.next().await {
            match data {
                Message::Text(msg) => {
                    // ???????????? socket ?????????????????????string???????????? Msg ??????
                    handle_message(msg.as_str().try_into().unwrap(), state1.clone().0).await;
                }
                _ => {}
            }
        }
    });

    let mut send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            // Msg ???????????? string
            let data = msg.as_ref().try_into().unwrap();
            // ?????? ws::Message???????????? socket ?????? client
            if sender.send(Message::Text(data)).await.is_err() {
                warn!("failed to send message");
                break;
            }
        }
    });

    tokio::select! {
        _ = &mut recv_task => send_task.abort(),
        _ = &mut send_task => recv_task.abort(),
    }

    warn!("connection closed");

    // this users has left, should send a leave msg to all users in this room
    let username = "fake_user";
    for room in state.get_user_rooms(username) {
        if let Err(e) = state.0.tx.send(Arc::new(Msg::leave(&room, username))) {
            warn!("failed to send leave msg: {e}");
        }
    }
}

async fn handle_message(msg: Msg, state: Arc<State>) {
    let msg = match msg.data {
        MsgData::Join => {
            // to avoid "borrow after move" error
            let room = msg.room.clone();
            let username = msg.username.clone();
            state
                .user_rooms
                .entry(username.clone())
                .or_insert_with(DashSet::new)
                .insert(room.clone());
            state
                .room_users
                .entry(room)
                .or_insert_with(DashSet::new)
                .insert(username);
            msg
        }
        MsgData::Leave => {
            // ????????????????????????????????????
            if let Some(v) = state.user_rooms.get(&msg.username) {
                // ???????????????????????????????????????????????????
                // ?????????????????????????????????????????????????????????????????????????????????
                // ??????????????? state.user_rooms ??????????????????????????? rooms ?????? DashSet ??????
                v.remove(&msg.room);
                if v.is_empty() {
                    // ?????????????????????????????????
                    drop(v);
                    // ??????????????????????????????????????????????????????????????????
                    state.user_rooms.remove(&msg.username);
                }
            }

            if let Some(v) = state.room_users.get(&msg.room) {
                v.remove(&msg.username);
                if v.is_empty() {
                    drop(v);
                    state.room_users.remove(&msg.room);
                }
            }
            msg
        }
        MsgData::Message(_) => msg,
    };

    // ??????????????? channel ???
    if let Err(e) = state.tx.send(Arc::new(msg)) {
        warn!("error sending message: {:?}", e);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use fake_socket::*;

    #[tokio::test]
    async fn handle_socket_should_work() -> Result<()> {
        let (_client1, _client2, state) = prepare_connections().await?;
        // verify state
        let mut users = state.get_room_users("lobby");
        users.sort();
        assert_eq!(users, &["alice", "linuxfish"]);

        let rooms = state.get_user_rooms("linuxfish");
        assert_eq!(rooms, &["lobby"]);

        Ok(())
    }

    #[tokio::test]
    async fn handle_message_and_leave_should_work() -> Result<()> {
        let (mut client1, mut client2, state) = prepare_connections().await?;

        let msg1 = &Msg::message(
            "lobby",
            "linuxfish",
            MsgData::Message("hello world".to_string()),
        );
        client1.send(Message::Text(msg1.try_into()?))?;
        // self should received
        verify(
            &mut client1,
            "lobby",
            "linuxfish",
            MsgData::Message("hello world".to_string()),
        )
        .await?;

        // alice should also received
        verify(
            &mut client2,
            "lobby",
            "linuxfish",
            MsgData::Message("hello world".to_string()),
        )
        .await?;

        let msg2 = &Msg::leave("lobby", "linuxfish");
        client1.send(Message::Text(msg2.try_into()?))?;

        assert!(client1.recv().await.is_some());
        assert!(client2.recv().await.is_some());

        // verify state
        let mut users = state.get_room_users("lobby");
        users.sort();
        assert_eq!(users, &["alice"]);

        let rooms = state.get_user_rooms("linuxfish");
        assert!(rooms.is_empty());

        Ok(())
    }

    async fn prepare_connections() -> Result<(FakeClient<Message>, FakeClient<Message>, ChatState)>
    {
        let (mut client1, socket1) = create_fake_connection();
        let (mut client2, socket2) = create_fake_connection();
        let state = ChatState::new();

        let state1 = state.clone();
        tokio::spawn(async move {
            handle_socket(socket1, state1).await;
        });

        let state2 = state.clone();
        tokio::spawn(async move {
            handle_socket(socket2, state2).await;
        });
        // client1 send a msg
        let msg1 = &Msg::join("lobby", "linuxfish");
        client1.send(Message::Text(msg1.try_into()?))?;

        // lcient2 send a msg
        let msg2 = &Msg::join("lobby", "alice");
        client2.send(Message::Text(msg2.try_into()?))?;

        // client1 and client2 should both receive this msg
        verify(&mut client1, "lobby", "linuxfish", MsgData::Join).await?;
        verify(&mut client2, "lobby", "linuxfish", MsgData::Join).await?;

        assert!(client1.recv().await.is_some());
        assert!(client2.recv().await.is_some());

        Ok((client1, client2, state))
    }

    async fn verify(
        client: &mut FakeClient<Message>,
        room: &str,
        username: &str,
        data: MsgData,
    ) -> Result<()> {
        if let Some(Message::Text(msg)) = client.recv().await {
            let msg = Msg::try_from(msg.as_str())?;
            assert_eq!(msg.room, room);
            assert_eq!(msg.username, username);
            assert_eq!(msg.data, data);
        }
        Ok(())
    }
}
