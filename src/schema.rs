table! {
    chats (id) {
        id -> Int8,
    }
}

table! {
    queue_element (el_name, queue_id, chat_id) {
        el_name -> Varchar,
        queue_id -> Int8,
        chat_id -> Int8,
        queue_place -> Int4,
    }
}

table! {
    queues (id, chat_id) {
        id -> Int8,
        chat_id -> Int8,
    }
}

joinable!(queue_element -> chats (chat_id));
joinable!(queues -> chats (chat_id));

allow_tables_to_appear_in_same_query!(
    chats,
    queue_element,
    queues,
);
