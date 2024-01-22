
struct SplitResult {
  int amount;
  String data[10];
};

SplitResult sr;

void setup() {
  Serial.begin(2000000);
  while(Serial.read() != 0x0A);
  Serial.println("+PICO_GPIO V1.0");
  sr = split("", " ", 1);
}

String line;

void loop() {
  if(!Serial.available()) return;
  int b = Serial.read();
  if(b != -1) {
    line += (char) b;
  } else {
    return;
  }
  if(line.endsWith("\n")) {
    line.trim();
    exec(line);
    Serial.println("!OK");
    line = "";
  }
}

void exec(String command) {
  sr = split(command, " ", 2);
  String base = sr.data[0];
  String arg = sr.amount == 2 ? sr.data[1] : "";
  if(base.equalsIgnoreCase("out")) {
    SplitResult args = split(arg, "=", 2);
    if(args.amount != 2) {
      Serial.println("!ERROR:No args");
      return;
    }
    int gpio = args.data[0].toInt();
    int bit = args.data[1].toInt();
    pinMode(gpio, OUTPUT);
    digitalWrite(gpio, bit);
    Serial.println(args.data[0] + "=" + digitalRead(gpio));
    return;
  }
  if(base.equalsIgnoreCase("outf")) {
    SplitResult args = split(arg, "=", 2);
    if(args.amount != 2) {
      Serial.println("!ERROR:No args");
      return;
    }
    int gpio = args.data[0].toInt();
    int bit = args.data[1].toInt();
    digitalWrite(gpio, bit);
    return;
  }
  if(base.equalsIgnoreCase("in^")) {
    int gpio = arg.toInt();
    pinMode(gpio, INPUT_PULLUP);
    Serial.println(arg + "=" + digitalRead(gpio));
    return;
  }
  if(base.equalsIgnoreCase("in")) {
    int gpio = arg.toInt();
    pinMode(gpio, INPUT_PULLDOWN);
    Serial.println(arg + "=" + digitalRead(gpio));
    return;
  }
  if(base.equalsIgnoreCase("float")) {
    int gpio = arg.toInt();
    pinMode(gpio, INPUT);
    Serial.println("~" + arg + "=" + digitalRead(gpio));
    return;
  }
  if(base.equalsIgnoreCase("inares")) {
    int f = arg.toInt();
    analogReadResolution(f);
    Serial.println("!INARES:" + String(f));
    return;
  }
  if(base.equalsIgnoreCase("ina")) {
    int gpio = arg.toInt();
    pinMode(gpio, INPUT);
    Serial.println(String("/") + arg + "=" + analogRead(gpio));
    return;
  }
  if(base.equalsIgnoreCase("pwmfreq")) {
    long f = arg.toInt();
    analogWriteFreq(f);
    Serial.println("!PWMFREQ:" + String(f));
    return;
  }
  if(base.equalsIgnoreCase("pwmres")) {
    int f = arg.toInt();
    analogWriteResolution(f);
    Serial.println("!PWMRES:" + String(f));
    return;
  }
  if(base.equalsIgnoreCase("pwm")) {
    SplitResult args = split(arg, "=", 2);
    if(args.amount != 2) {
      Serial.println("!ERROR:No args");
      return;
    }
    int gpio = args.data[0].toInt();
    int amount = args.data[1].toInt();
    pinMode(gpio, OUTPUT);
    analogWrite(gpio, amount);
    Serial.println(String("#") + args.data[0] + "=" + amount);
    return;
  }
  if(base.equalsIgnoreCase("pwmf")) {
    SplitResult args = split(arg, "=", 2);
    if(args.amount != 2) {
      Serial.println("!ERROR:No args");
      return;
    }
    int gpio = args.data[0].toInt();
    int amount = args.data[1].toInt();
    analogWrite(gpio, amount);
    return;
  }
  if(base.equalsIgnoreCase("pwmstream")) {
    int gpio = arg.toInt();
    int amount = 0;
    Serial.println("!STREAMING:" + String(gpio));
    pinMode(gpio, OUTPUT);
    analogWriteResolution(8);
    while(true) {
      if((amount = Serial.read()) != -1) {
        analogWrite(gpio, amount);
      }
    }
    return;
  }
  if(base.equalsIgnoreCase("audiostream")) {
    int gpio = arg.toInt();
    long to_skip = 0;
    int amount = 0;
    Serial.println("!STREAMING:" + String(gpio));
    pinMode(gpio, OUTPUT_12MA);
    analogWriteResolution(8);
    while(true) {
      if((amount = Serial.read()) != -1) {
        analogWrite(gpio, amount);
      }
      to_skip = max(to_skip - 1, 0);
      if(Serial.available() >= 245) {
        to_skip = min(to_skip + 2, 8192 * 16);
      }
      for(int i = 0; i * 16 < to_skip; i++) {
        while(Serial.read() == -1);
      }
    }
    return;
  }
  // it's always going to run on this, but just in case someone wants to adapt it, i have gated this
  #ifdef ARDUINO_ARCH_RP2040
  if(command.equalsIgnoreCase("reset")) {
    Serial.println("!RESET");
    rp2040.reboot();
    return;
  }
  #else
  if(command.equalsIgnoreCase("reset")) {
    Serial.println("!RESET");
    Serial.end();
    for(int i = 0; i < 256; i++) {
      pinMode(i, INPUT);
    }
    setup();
    return;
  }
  #endif
  if(command == "") {
    return;
  }
  Serial.println("!ERROR:No such command");
}

struct SplitResult split(String s, String splitter, int maxAmount) {
  int index;
  SplitResult sr = SplitResult {
    amount: 0,
    data: {}
  };
  while((index = s.indexOf(splitter)) != -1 && sr.amount < maxAmount - 1) {
    sr.data[sr.amount] = s.substring(0, index);
    s = s.substring(index + 1, s.length());
    sr.amount++;
  }
  if(maxAmount != 0) {
    sr.data[sr.amount++] = s;
  }
  return sr;
}
