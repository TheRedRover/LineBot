-- Your SQL goes here

create table chats (
    id bigint primary key
);

create table queues (
    id bigint,
    chat_id bigint,
    primary key (id, chat_id)
);

create table queue_element (
    el_name varchar(200) not null,
    queue_id bigint,
    chat_id bigint,
    queue_place integer not,
    primary key(el_name, queue_id, chat_id)
);

alter table queues add foreign key (chat_id) 
    references chats(id);

alter table queue_element add foreign key (queue_id, chat_id) 
    references queues(id, chat_id);
alter table queue_element add foreign key (chat_id) 
    references chats(id);
