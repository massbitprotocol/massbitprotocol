#!/usr/bin/env python3
import os
import smtplib
import subprocess
import time


def send_email(text,sent_to):
    # =============================================================================
    # SET EMAIL LOGIN REQUIREMENTS
    # =============================================================================
    gmail_user = 'codelightnotify@gmail.com'
    gmail_app_password = 'zerdsumxzyqhsnzc'

    # =============================================================================
    # SET THE INFO ABOUT THE SAID EMAIL
    # =============================================================================
    sent_from = gmail_user
    sent_subject = "Running report!"
    sent_body = (text)

    email_text = """\
From: %s
To: %s
Subject: %s

%s
""" % (sent_from, ", ".join(sent_to), sent_subject, sent_body)

    # =============================================================================
    # SEND EMAIL OR DIE TRYING!!!
    # =============================================================================
    success = False

    while success is False:
        try:
            server = smtplib.SMTP_SSL('smtp.gmail.com', 465)
            server.ehlo()
            server.login(gmail_user, gmail_app_password)
            server.sendmail(sent_from, sent_to, email_text)
            server.close()
            success = True
            print('Email sent!')
        except Exception as exception:
            print("Error: %s!\n\n" % exception)
            time.sleep(1)

def get_file_size(file):
    if os.path.isfile(file):
        return os.path.getsize(file)
    else:
        print("File is not exist!")
        return 0


if __name__ == '__main__':
    sent_to = ['anhhuy0501@gmail.com', 'codelightnotify@gmail.com', 'vuviettai@gmail.com', 'phanthanhhuy1996@gmail.com', 'nguyenmanhdat2903@gmail.com']
    text = "The test stopped running"
    chain_reader = "../log/chain-reader.log"
    index_manager = "../log/index-manager.log"
    last_chain_reader_size=0
    last_index_manager_size=0
    while True:
        chain_reader_size = get_file_size(chain_reader)
        index_manager_size = get_file_size(index_manager)

        print(f"chain_reader_size: {chain_reader_size}")
        print(f"index_manager_size: {index_manager_size}")


        if index_manager_size == last_index_manager_size or chain_reader_size == last_index_manager_size:
            print("Stopping")
            send_email(text, sent_to)
            exit()
        else:
            last_chain_reader_size = chain_reader_size
            last_index_manager_size = index_manager_size
            print("Running")

        time.sleep(300)
