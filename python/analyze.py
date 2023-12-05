import datetime
import json

import matplotlib as mpl
import matplotlib.pyplot as plt

times = []
targets = []
actual = []
modulations = []
outside_temps = []

BOILER_BTU = 155000


def btu_to_mcf(btu):
    return btu / 1039000.0


now = datetime.datetime.utcnow()

btu_used = 0
prev_time = None
start_time = None
last_time = None

with open("log.json") as f:
    for line in f:
        j = json.loads(line)
        dt = datetime.datetime.strptime(j["time"], "%Y-%m-%dT%H:%M:%S")
        if (now - dt).total_seconds() > 60 * 60 * 24:
            continue
        print(
            dt,
            j["boiler_modulation_rate"],
            j["boiler_target_temp"],
            j["output_temp"],
            j["outdoor_temp"],
        )
        times.append(dt)
        tar = j["boiler_target_temp"]
        if tar > 120:
            tar = 0
        targets.append(tar)
        outside_temps.append(j["outdoor_temp"])
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

# Plot with mpl / plt

mpl.rcParams["timezone"] = "EST"
fig, ax = plt.subplots()
ax.plot(times, targets, label="Target")
ax.plot(times, actual, label="Actual")
ax.plot(times, modulations, label="Modulation")
ax.plot(times, outside_temps, label="Outside Temp")
ax.set_xlabel("Time")
ax.set_ylabel("Temperature")
ax.legend()
fig.autofmt_xdate()
plt.show()
