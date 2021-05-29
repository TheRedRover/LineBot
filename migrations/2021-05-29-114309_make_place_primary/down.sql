-- This file should undo anything in `up.sql`
alter table queue_elements drop constraint queue_elements_pkey;

alter table queue_elements add primary key(element_name, queue_id, chat_id);
