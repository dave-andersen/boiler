import asyncio
import json
import pymodbus
import pymodbus.client
import datetime
import time

datafields = []

SERIAL="/dev/serial0"
DEVICE = 250


class DataField:
    def __init__(self, name, address, length, data_type, degree_wrap=False):
        self.name = name
        self.address = address
        self.length = length
        self.data_type = data_type
        self.degree_wrap = degree_wrap
        global datafields
        datafields.append(self)


df_output_temp = DataField("output_temp", 0x9106, 1, "uint8")
df_boiler_target_temp = DataField("boiler_target_temp", 0x9109, 1, "uint8")
df_boiler_status = DataField("boiler status", 0x9105, 1, "uint8")
df_outdoor_temp = DataField("outdoor_temp", 0x9112, 1, "uint8", degree_wrap=True) # note negative handling
df_config_supply_max = DataField("supply_max", 0x9120, 1, "uint8")
df_config_supply_min = DataField("supply_min", 0x9121, 1, "uint8")
df_config_odr_max = DataField("odr_max", 0x9122, 1, "uint8")
df_config_odr_min = DataField("odr_min", 0x9123, 1, "uint8", degree_wrap=True)
df_config_boiler_max = DataField("boiler_max", 0x9124, 1, "uint8")
df_boier_out_1 = DataField("boiler_out_1_temp", 0x9180, 1, "uint8")
df_boiler_in_temp = DataField("boiler_in_temp", 0x9188, 1, "uint8")
df_input_status = DataField("input_status", 0x918B, 1, "uint8")
df_flue_temp = DataField("flue_temp_1", 0x918C, 1, "uint8")
df_supply_temp = DataField("local_supply_temp", 0x918D, 1, "uint8")
df_return_temp = DataField("local_return_temp", 0x9196, 1, "uint8")
df_modulation_rate = DataField("boiler_modulation_rate", 0x9232, 1, "uint8")



async def read_datafield(client, datafield):
    rr = await client.read_holding_registers(
        datafield.address, datafield.length, slave=DEVICE
    )
    #print(f"{datafield.name}: ", rr.registers)
    v = rr.registers[0]
    if datafield.degree_wrap and v > 232:
        v = v - 256
    return v


async def main():
    with open("log.json", "a") as f:
      while True:
        client = pymodbus.client.AsyncModbusSerialClient(
            port=SERIAL, baudrate=19200, stopbits=1, parity="N"
        )
    
        await client.connect()
        assert client.connected

        status = { df.name : await read_datafield(client, df) for df in datafields }
        status["time"] = datetime.datetime.utcnow().replace(microsecond=0).isoformat()
        json.dump(status, f)
        f.write("\n")
        f.flush()
    

        client.close()
        time.sleep(60)


asyncio.run(main())

