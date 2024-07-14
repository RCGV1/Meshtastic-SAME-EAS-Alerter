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

The Meshtastic SAME EAS Alerter is a lightweight tool designed to forward warnings, emergencies, or statements sent over the air to a local Meshtastic network. It operates without needing a WiFi connection. The setup involves connecting the hosting computer to an RTL SDR via USB and to a running Meshtastic node via serial or TCP.

![Flood Example](./images/Flood.jpg)

## 💿 Installation
Installation example for Raspbian
1. Install rtl_fm.  
   follow these [instructions](https://fuzzthepiguy.tech/rtl_fm-install/)

2. Install Meshtastic-SAME-EAS-Alerter
````
# Download the updated .deb file
wget https://github.com/RCGV1/Meshtastic-SAME-EAS-Alerter/releases/download/v<VERSION_NUMBER_HERE>/Meshtastic-SAME-EAS-Alerter_<VERSION_NUMBER_HERE>_arm64.deb

# Install the .deb package
sudo dpkg -i Meshtastic-SAME-EAS-Alerter_<VERSION_NUMBER_HERE>_arm64.deb

# Fix any dependency issues
sudo apt-get install -f
````

Other operating systems may have a different installation


## 📻 Meshtastic Setup
- All alerts will be sent to the primary channel (channel number 0)       
- Set a secondary channel (channel 1) to be the logger channel, either by creating a different name for it or some sort of encryption   
  - All test alerts will be sent to channel 1 so be sure to configure it   
- The connected node should have a great line of sight of other nodes to effectively relay alerts  


## 🖋️ Usage

### RTL FM input
- You must pass in the input from an rtl fm stream
- For a detailed installation guide of rtl_fm check out the Installation instructions
- Set the desired frequency to the nearest National Weather Service radio station typically in the 162.40 to 162.55 MHz range
```
rtl_fm -f <FREQUENCY_IN_HZ_HERE> -s 48000 -r 48000 | Meshtastic-SAME-EAS-Alerter
```
> The frequency input is in Hz not MHz

### help
```
Meshtastic-SAME-EAS-Alerter --help
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

### host
- Enter the address of the TCP host to connect to, in the form of IP:PORT
```
Meshtastic-SAME-EAS-Alerter --host <TCP_HOST_HERE>
```

### alert channel  
- Input the channel number that alerts will be sent to  
- By default, (if nothing is provided) alerts will be sent to channel number 0  
```
Meshtastic-SAME-EAS-Alerter --alert-channel <CHANNEL_NUMBER_HERE>
```

### test channel  
- Input the channel number that test messages will be sent to  
  - Usually test messages are transmitted weekly from the weather service station  
  - These messages can help you tell if your system is working but they can be annoying as they happen often  
- By default, (if nothing is provided) test messages will be ignored  
```
Meshtastic-SAME-EAS-Alerter --test-channel <CHANNEL_NUMBER_HERE>
```

### Full example
- You need both a Meshtastic serial port passed as an arg and rtl fm to run this
- Alert channel and test channel are optional (see above)
````
rtl_fm -f <FREQUENCY_IN_HZ_HERE> -s 48000 -r 48000 | Meshtastic-SAME-EAS-Alerter --port <MESHTASTIC_PORT_HERE> --alert-channel <CHANNEL_NUMBER_HERE> --test-channel <CHANNEL_NUMBER_HERE>
````
or
````
rtl_fm -f <FREQUENCY_IN_HZ_HERE> -s 48000 -r 48000 | Meshtastic-SAME-EAS-Alerter --host <TCP_HOST_HERE> --alert-channel <CHANNEL_NUMBER_HERE> --test-channel <CHANNEL_NUMBER_HERE>
````
> Remember to replace the placeholder values

# 📓 TODO
PRs are welcome
- [ ] Canadian Location Codes and marine
- [x] Connect to a node over TCP
- [ ] Live audio to text transcription for forwarding instructions sent after the SAME header

# 📇 Contact
If you need assistance deploying an alerter feel free to contact me
- Discord: RCGV_
- Email: Benjaminfaer@gmail.com

