class Temps:
    def __init__(self, odr_min, odr_max, supply_min, supply_max):
        self.odr_min = odr_min
        self.odr_max = odr_max
        self.supply_min = supply_min
        self.supply_max = supply_max


conf_old = Temps(-13, 68, 86, 165)
conf_new = Temps(-14, 67, 85, 168)
conf_new2 = Temps(-14, 66, 85, 169)
conf_new3 = Temps(-14, 64, 85, 171)
conf_new4 = Temps(-14, 63, 85, 175)


def f(t, c):
    if t > c.odr_max:
        return c.supply_min
    if t < c.odr_min:
        return c.supply_max
    percent = (c.odr_max - t) / (c.odr_max - c.odr_min)
    return c.supply_min + percent * (c.supply_max - c.supply_min)


for c in [conf_old, conf_new, conf_new2, conf_new3, conf_new4]:
    for degree in [49, 44, 41, 39, 29, 19]:
        print(degree, f(degree, c))
    print()
