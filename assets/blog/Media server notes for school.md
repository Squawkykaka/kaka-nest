---
date: 2025-08-25
published: true
title: Media server notes for school
read_mins: 5
tags:
  - "#school"
  - "#mediaserver"
  - "#test"
---

Media Server Assignment

Task Outline

Components of a Media Server
-Hardwware
CPU
Software
network connections
ethernet
ports
wifi 


| Component | Porpoise                                                                                                                                                               |
| --------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| CPU       | Runs the calcualtions and is the brains of the computer                                                                                                                |
| WIFI      | allows you to connect to a network and talk to other devices, needed on the raspberry pis as the teacher says we cannot use the ethernet                               |
| Linux     | operating system that jellyfin runs on, used because windows bad and the best option for servers                                                                       |
| Docker    | A CME (Container Management Engine) which runs mini virtualised enviroments that contain preconfigured systems. <br>e.g. jellyfin dependancies and files in one place. |
| Command   | A string of text you type into a terminal in order to do something on your computer                                                                                    |
| Router    | A device that routes network connections between devices                                                                                                               |



Explain what each of these are, what is their function in the media server
Point 2 in the Task list


HIghlight the functional testing
 - can I find the server
 - can i login to the server
 - can i see a list of things on the server
 - can i play the thing
 - can I stop the ting
 - can aI pause the thing


A lot of tests related to jelly-fin directly are unnecessary, as jellyfin is a product used by many people with extensive tests, i only need to test integration with other parts of the system.

| Test                                       | Process                                                                                                                              | Success |
| ------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------ | ------- |
| Can i login                                | go to the raspberry pi's ip, and login with user and pass `comp13`, if it redirects you to the movie page, success                   | ✅       |
| Failing to login to admin 3 times bans you | go to the login page with user `admin` and login randomly 3 times, if you relaod the page after that and are banned, success         | ✅       |
| Ssh works successfully                     | go to the servers monitor, find the ip with `ip a` and attempt to ssh into it with `ssh comp13@ip-address` and if you login, success | ✅       |
|                                            |                                                                                                                                      |         |


User requirements - Are
can I access/login  and can I watch the movies

During testing I found xcyz and did abc to fix it

I've done lots of these = - although not documented in this I have refined what I di by doing abx, dfg, retygh, ert, etr

