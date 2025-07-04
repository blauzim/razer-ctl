import pandas as pd
import numpy as np

# Wireshark filter for the Razer commands
# (_ws.col.protocol == "USBHID" ) && (_ws.col.info == "SET_REPORT Request") && usb.data_len > 20

raw_data = pd.read_csv('data/test.csv', sep=',')
annotations = pd.read_csv('data/annotations2.csv', sep=' ', header=None)

table = [['action', 'cmd', 'argc', 'arg0', 'arg1', 'arg2', 'arg3']]

# test debug a single entry
if False :
    index = 0 
    annocation = annotations.iloc[index]
    description = "set balanced manual fan mode"

for (index, annotation) in annotations.iterrows():
    time_s = annotation[0]
    description = annotation[1]
    usb_frames = (raw_data[raw_data["Time"].astype(int) == time_s])["Data"]
    row = [description]
    assert len(usb_frames)
    for frame in usb_frames:
        argc = int(frame[10:12], 16)
        cmd = frame[12:16]
        row += [cmd, argc]
        for i in range(argc):
            row += [frame[(16 + 2 * i):(16 + 2 * i + 2)]]
        table.append(row)
        row = ['']

print(pd.DataFrame(table))

pd.DataFrame(table).to_clipboard()
