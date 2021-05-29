table! {
    /// Representation of the `chats` table.
    ///
    /// (Automatically generated by Diesel.)
    chats (id) {
        /// The `id` column of the `chats` table.
        ///
        /// Its SQL type is `Int8`.
        ///
        /// (Automatically generated by Diesel.)
        id -> Int8,
    }
}

table! {
    /// Representation of the `queue_elements` table.
    ///
    /// (Automatically generated by Diesel.)
    queue_elements (queue_place, queue_id, chat_id) {
        /// The `element_name` column of the `queue_elements` table.
        ///
        /// Its SQL type is `Varchar`.
        ///
        /// (Automatically generated by Diesel.)
        element_name -> Varchar,
        /// The `queue_id` column of the `queue_elements` table.
        ///
        /// Its SQL type is `Int8`.
        ///
        /// (Automatically generated by Diesel.)
        queue_id -> Int8,
        /// The `chat_id` column of the `queue_elements` table.
        ///
        /// Its SQL type is `Int8`.
        ///
        /// (Automatically generated by Diesel.)
        chat_id -> Int8,
        /// The `queue_place` column of the `queue_elements` table.
        ///
        /// Its SQL type is `Int4`.
        ///
        /// (Automatically generated by Diesel.)
        queue_place -> Int4,
    }
}

table! {
    /// Representation of the `queues` table.
    ///
    /// (Automatically generated by Diesel.)
    queues (id, chat_id) {
        /// The `id` column of the `queues` table.
        ///
        /// Its SQL type is `Int8`.
        ///
        /// (Automatically generated by Diesel.)
        id -> Int8,
        /// The `chat_id` column of the `queues` table.
        ///
        /// Its SQL type is `Int8`.
        ///
        /// (Automatically generated by Diesel.)
        chat_id -> Int8,
        /// The `qname` column of the `queues` table.
        ///
        /// Its SQL type is `Nullable<Text>`.
        ///
        /// (Automatically generated by Diesel.)
        qname -> Nullable<Text>,
    }
}

joinable!(queue_elements -> chats (chat_id));
joinable!(queues -> chats (chat_id));

allow_tables_to_appear_in_same_query!(chats, queue_elements, queues,);
