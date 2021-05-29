-- This file should undo anything in `up.sql`

alter table queue_elements drop constraint queue_elements_pkey;

alter table queue_elements add primary key(queue_place, queue_id, chat_id);
