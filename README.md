# Meshtastic SAME EAS Alerter


<a href="https://www.weather.gov/" class="image-container">
    <img src="https://upload.wikimedia.org/wikipedia/commons/thumb/f/ff/US-NationalWeatherService-Logo.svg/2048px-US-NationalWeatherService-Logo.svg.png" width=100>
</a>

<a href="https://www.fema.gov/emergency-managers/practitioners/integrated-public-alert-warning-system/public/emergency-alert-system" class="image-container">
    <img src="https://upload.wikimedia.org/wikipedia/commons/1/15/EAS_new.svg" width=120>
</a>

<a href="https://Meshtastic.org" class="image-container">
    <img src="https://github.com/meshtastic/design/blob/master/Meshtastic%20Powered%20Logo/M-POWERED.png?raw=true" width=100>
</a>

The Meshtastic SAME EAS Alerter is a lightweight tool designed to forward warnings, emergencies, or statements sent over the air to a local Meshtastic network. It operates without needing a WiFi connection. The setup involves connecting the hosting computer to an RTL SDR via USB and to a running Meshtastic node via serial.


## 💿 Installation
COMING SOON

## 🖋️ Usage

### RTL FM input
- You must pass in the input from an rtl fm stream
- For a detailed installation guide of rtl_fm check out the Installation instructions
- Set the desired frequency to the nearest National Weather Service radio station typically in the 162.40 to 162.55 MHz range
```
rtl_fm -f <FREQUENCY_IN_HZ_HERE> -s 48000 -r 48000 | Meshtastic-SAME-EAS-Alerter
```

### ports
- Find all available serial ports
- Find the port your Meshtastic node is connected to

```
Meshtastic-SAME-EAS-Alerter --ports
```

### port

```
Meshtastic-SAME-EAS-Alerter --port <MESHTASTIC_PORT_HERE>
```

### Full example
You need both a Meshtastic serial port passed as an arg and rtl fm to run this
````
rtl_fm -f <FREQUENCY_IN_HZ_HERE> -s 48000 -r 48000 | Meshtastic-SAME-EAS-Alerter --port <MESHTASTIC_PORT_HERE>
````