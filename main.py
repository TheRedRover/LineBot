import random

import telebot

import os

from datetime import datetime

token = '<TOKEN>'

if __name__ == '__main__':


    bot = telebot.TeleBot(token)


    @bot.message_handler(commands=['start'])
    def send_message(message):
        print("hello mess", message.chat.id)
        bot.reply_to(message, "Hello")

    @bot.message_handler(commands=['create_queue'])
    def send_message(message):
        bot.reply_to(message, "send command with list of your group.txt")


    @bot.message_handler(func=lambda message: message.document.mime_type == 'text/plain', content_types=['document'])
    def send_message(message):
        print("got request", message.chat.id)
        try:
            random.seed()
            file_name = message.document.file_name
            if(not file_name=="group.txt"):
                bot.send_message(message.chat.id, "File must be called 'group.txt'")
                return
            file_id_info = bot.get_file(message.document.file_id)
            print("got document", message.chat.id)
            downloaded_file = bot.download_file(file_id_info.file_path)
            src = file_name
            with open(src, 'wb') as loaded_file:
                loaded_file.write(downloaded_file)
            with open(src, 'r') as file:
                lines = file.read()
                lines = list(lines.split('\n'))
                # random.shuffle(lines)
                # random.shuffle(lines)
                random.shuffle(lines)
                answer = ""
                i=1
                for line in lines:
                    answer += str(i)+ " - "+line+'\n'
                    i+=1
                bot.send_message(message.chat.id, answer)
            os.remove(src)
        except Exception as ex:
            print(ex)


    bot.polling()


