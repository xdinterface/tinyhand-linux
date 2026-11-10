SECONDS=0
printf 'sound = false\ntray = false\nink-fade = 0\n' > $TINYHAND_CONFIG
TH_TIMEOUT=60 th > $E/th.log 2>&1 &
sleep 1
cur 300 300
th on; sleep 0.5
drag 400 300 900 500 20; sleep 3.2
cur 600 500; sleep 0.6
shot s5-a-toolbar-hidden
cur 714 915; sleep 0.3; down; up; sleep 0.3
grep -E "^pointer" $TINYHAND_CONFIG
cur 817 915; sleep 0.3; down; up; sleep 0.5
cur 760 800; sleep 0.3
shot s5-b-inktray
cur 652 868; sleep 0.2; down; up; sleep 0.4; grep -E "^ink" $TINYHAND_CONFIG
drag 300 600 700 650 10; sleep 0.2
key -s 200 -P Alt_L -s 700 -p Alt_L & KEY=$!
sleep 0.5; shot s5-c-alt-held; wait $KEY; sleep 0.3
cur 650 450; sleep 0.3; shot s5-d-alt-released
cur 860 915; sleep 0.2; cur 864 915; sleep 0.3; down; up; sleep 0.3; grep -E "^sound" $TINYHAND_CONFIG
cur 910 915; sleep 0.3; down; up; sleep 0.4
th status; echo "after-exit-button=$?"
th quit
cat $E/th.log | tail -5
echo "took ${SECONDS}s"
