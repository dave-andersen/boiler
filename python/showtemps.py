import datetime
import json

import matplotlib as mpl
import matplotlib.pyplot as plt

now = datetime.datetime.now()

times = []
temps = []

with open("temp.log") as f:
    for line in f:
        if not line.startswith("{"):
            continue
        j = json.loads(line)
        timestr = j["date"].split(".")[0]
        try:
            dt = datetime.datetime.strptime(timestr, "%Y-%m-%dT%H:%M:%SZ")
        except Exception:
            dt = datetime.datetime.strptime(timestr, "%Y-%m-%d %H:%M:%S") # 2023-12-06 13:02:08.627029146
        if (now - dt).total_seconds() > 60 * 60 * 24:
            continue
        print(
            dt,
            j["temp"])
        times.append(dt)
        temps.append(j["temp"])

mpl.rcParams["timezone"] = "EST"
fig, ax = plt.subplots()
ax.plot(times, temps, label="Indoor Temperature")
ax.set_xlabel("Time")
ax.set_ylabel("Temperature")
ax.legend()
fig.autofmt_xdate()
plt.show()
