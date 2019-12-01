<h1>piClock</h1>
<h2>Summary</h2>
<p>piClock Provides a program written in Rust to display the current time on a <a href="https://thepihut.com/products/zeroseg">ZeroSeg</a> device attached to a <a href="https://www.raspberrypi.org/products/raspberry-pi-zero-w/">Raspberry Pi Zero W</a>.
<p>The program displays the time in the format "hh-mm-ss" and uses the two buttons on the ZeroSeg board to allow the user to:
<ul>
<li>Change the orientation of the display (normal or inverted)</li>
<li>Change the brightness of the display in incremental steps</li>
</ul>
<p>The program can be initiated on boot with the following entry in <code>/lib/systemd/system/piClock.service</code>:
<figure><pre>
[Unit]
Description=piClock Service
After=multi-user.target
<br>
[Service]
Type=idle
ExecStart=/home/dave/projects/piClock/target/debug/piClock &>> /home/dave/piClock-daemon.output
<br>
[Install]
WantedBy=multi-user.target
</pre></figure>
<p>where the ExecStart parameter points to the appropriate executable.  Then, before rebooting, enter the commands:
<figure>
    <pre>
        <code>
sudo systemctl daemon-reload
sudo systemctl enable piClock.service
        </code>
    </pre>
</figure>
