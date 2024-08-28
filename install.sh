# !/bin/bash

# Replace "USER_NAME" with your username from the ./headsetcontrol-notifyd.service file
USER_NAME=$(whoami)
sed -i "s/USER_NAME/$USER_NAME/g" ./headsetcontrol-notifyd.service

sudo cp ./headsetcontrol-notifyd.service /etc/systemd/user/headsetcontrol-notifyd.service

systemctl --user daemon-reload
systemctl --user enable --now headsetcontrol-notifyd.service
