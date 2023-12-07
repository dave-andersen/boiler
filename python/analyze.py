import datetime
import json

import matplotlib as mpl
import matplotlib.pyplot as plt
import numpy as np

times = []
targets = []
actual = []
modulations = []
outside_temps = []
indoor_temps = []
temp_times = []
return_temps = []

BOILER_BTU = 155000


def btu_to_mcf(btu):
    return btu / 1039000.0


now = datetime.datetime.utcnow()

btu_used = 0
prev_time = None
start_time = None
last_time = None
state = 0
last_start = None
last_stop = None

with open("log.json") as f:
    for line in f:
        if not line.startswith("{"):
            continue
        j = json.loads(line)
        timestr = j["time"].split(".")[0]
        try:
            dt = datetime.datetime.strptime(timestr, "%Y-%m-%dT%H:%M:%S")
        except Exception:
            dt = datetime.datetime.strptime(
                timestr, "%Y-%m-%d %H:%M:%S"
            )  # 2023-12-06 13:02:08.627029146
        if (now - dt).total_seconds() > 60 * 60 * 24:
            continue
        # print(
        #     dt,
        #     j["boiler_modulation_rate"],
        #     j["boiler_target_temp"],
        #     j["output_temp"],
        #     j["outdoor_temp"],
        # )
        if j["boiler_modulation_rate"] > 0:
            if state == 0:
                last_start = dt
                last_stop = None
            state = 1
        else:
            if state == 1:
                last_stop = dt
                duration_minutes = (last_stop - last_start).total_seconds() / 60.0
                print(f"Run duration: {duration_minutes:.2f}m")
            state = 0
        times.append(dt)
        tar = j["boiler_target_temp"]
        if tar > 120:
            tar = 0
        targets.append(tar)
        if "indoor_temp" in j and j["indoor_temp"] is not None:
            temp_times.append(dt)
            indoor_temps.append(j["indoor_temp"])
        outside_temps.append(j["outdoor_temp"])
        return_temps.append(j["local_return_temp"])
        actual.append(j["output_temp"])
        modulations.append(j["boiler_modulation_rate"])
        if prev_time is not None:
            btu_used += (
                BOILER_BTU
                * (j["boiler_modulation_rate"] / 100.0)
                * ((dt - prev_time).total_seconds() / 3600.0)
            )
        prev_time = dt
        if start_time is None:
            start_time = dt
        last_time = dt


mcf_used = btu_to_mcf(btu_used)
duration_hours = (last_time - start_time).total_seconds() / 3600.0
print(f"MCF of gas used over {duration_hours:.2f} hours: {mcf_used:.2f}")
last_dur = last_time - last_start
if last_stop is not None:
    last_dur = last_stop - last_start

print(f"Duration of last run: {(last_dur).total_seconds() / 60.0:.2f}m")

# Plot with mpl / plt

mpl.rcParams["timezone"] = "EST"
fig, ax = plt.subplots()
ax2 = ax.twinx()
ax.plot(times, targets, label="Target")
ax.plot(times, actual, label="Actual")
ax.plot(times, modulations, label="Modulation")
ax.plot(times, outside_temps, label="Outside Temp")
ax.plot(times, return_temps, label="Return Temp")
# indoor temps are a bit noisy, so smooth
indoor_temps = np.array(indoor_temps)
indoor_conv_smooth = 5
# for display convert indoor_temps to F
indoor_temps = indoor_temps * 9 / 5 + 32
indoor_temps = np.convolve(
    indoor_temps, np.ones(indoor_conv_smooth) / indoor_conv_smooth, mode="valid"
)
# Fix the boundaries\
temp_times = np.array(temp_times)
temp_times = temp_times[indoor_conv_smooth // 2 : -(indoor_conv_smooth // 2)]
ax2.plot(temp_times, indoor_temps, label="Indoor Temp", color="pink")
ax.set_xlabel("Time")
ax.set_ylabel("Temperature")
ax2.set_ylabel("Indoor Temp")
ax.legend()
fig.autofmt_xdate()
plt.show()
