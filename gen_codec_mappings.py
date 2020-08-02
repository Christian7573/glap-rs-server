import re
codec_file = "\n".join(open("./src/codec.rs", "r").readlines())

class ReadingName:
    def __init__(self, name, next):
        self.status = 0
        self.name = name
        self.next = next
    def progress(self, character):
        if character == self.name[self.status]:
            self.status += 1
            if self.status == len(self.name):
                return self.next
        else:
            self.status = 0
        return self

class CollectingBlock:
    def __init__(self, opening, closing, next):
        self.curly = 0
        self.in_string = 0
        self.escaped = False
        self.value = ""
        self.opening = opening
        self.closing = closing
        self.next = next
    def progress(self, character):
        if character == self.opening and not self.in_string:
            self.curly += 1
        elif character == self.closing and not self.in_string:
            self.curly -= 1
            if self.curly < 0:
                self.next.value = self.value
                return self.next
        self.value += character
        if character == '"' and self.in_string == 0:
            self.in_string = 1
        elif character == "'" and self.in_string == 0:
            self.in_string = 2
        elif character == '"' and not self.escaped and self.in_string == 1:
            self.in_string = 0
        elif character == "'" and not self.escaped and self.in_string == 2:
            self.in_string = 0
        elif character == '\\':
            self.escaped = not self.escaped
        return self

class HoldValue:
    def __init__(self):
        self.value = None
    def progress(self, character):
        return self

class CommentBypass:
    def __init__(self, next):
        self.next = next
        self.is_armed = False
    def progress(self, character):
        if character == "*":
            self.is_armed = True
        elif character == "/" and self.is_armed:
            return self.next
        else:
            self.is_armed = False
        return self

class NameExtraction:
    def __init__(self):
        self.names = []
        self.collection = ""
    def progress(self, character):
        if re.match("\\s", character):
            return self
        elif character == ",":
            self.names.append(self.collection)
            self.collection = ""
            return self
        elif character == "/":
            return CommentBypass(self)
        elif character == "(":
            return CollectingBlock("(",")",self)
        else:
            self.collection += character
            return self

progression = ReadingName("ToClientMsg {", CollectingBlock("{","}", HoldValue()))
for char in codec_file:
    progression = progression.progress(char)
if type(progression) != HoldValue:
    print("Unsuccessful in extracting ToClientMsg")
    exit()

to_client_msg = progression.value
progression = NameExtraction()
for char in to_client_msg:
    progression = progression.progress(char)
if type(progression) != NameExtraction:
    print("Unsuccessful in parsing ToClientMsg")
    exit()
if progression.collection != "":
    progression.names.append(progression.collection)

out = "export const FromServer = { "
for name in progression.names:
    out += name + r':{},'
out += " };\nFromServer.to_id = new Map([ "
for i in range(len(progression.names)):
    out += "[FromServer." + progression.names[i] + "," + str(i) + "],"
out += " ]);\nFromServer.from_id = new Map([ "
for i in range(len(progression.names)):
    out += "[" + str(i) + ",FromServer." + progression.names[i] + "],"
out += " ]);\n\n"

progression = ReadingName("FromClientMsg {", CollectingBlock("{","}", HoldValue()))
for char in codec_file:
    progression = progression.progress(char)
if type(progression) != HoldValue:
    print("Unsuccessful in extracting FromClientMsg")
    exit()

from_client_msg = progression.value
progression = NameExtraction()
for char in from_client_msg:
    progression = progression.progress(char)
if type(progression) != NameExtraction:
    print("Unsuccessful in parsing FromClientMsg")
    exit()
if progression.collection != "":
    progression.names.append(progression.collection)

out += "export const ToServer = { "
for name in progression.names:
    out += name + r':{},'
out += " };\nToServer.to_id = new Map([ "
for i in range(len(progression.names)):
    out += "[ToServer." + progression.names[i] + "," + str(i) + "],"
out += " ]);\nToServer.from_id = new Map([ "
for i in range(len(progression.names)):
    out += "[" + str(i) + ",ToServer." + progression.names[i] + "],"
out += " ]);\n\n"

outfile = open("./codec.js", "w")
outfile.write(out)
outfile.close()