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
        elif character == "{":
            return CollectingBlock("{","}",self)
        else:
            self.collection += character
            return self

def process_enum_mapping(opening_name, out_name):
    global codec_file
    progression = ReadingName(opening_name, CollectingBlock("{","}", HoldValue()))
    for char in codec_file:
        progression = progression.progress(char)
    if type(progression) != HoldValue:
        print("Unsuccessful in extracting", opening_name)
        exit()

    to_client_msg = progression.value
    progression = NameExtraction()
    for char in to_client_msg:
        progression = progression.progress(char)
    if type(progression) != NameExtraction:
        print("Unsuccessful in parsing", opening_name)
        exit()
    if progression.collection != "":
        progression.names.append(progression.collection)

    out = "export const %s = { " % out_name
    for name in progression.names:
        out += name + r':{},'
    out += " };\n%s.to_id = new Map([ " % out_name
    for i in range(len(progression.names)):
        out += "[%s.%s,%s]," % (out_name, progression.names[i], str(i))
    out += " ]);\n%s.from_id = new Map([ " % out_name
    for i in range(len(progression.names)):
        out += "[%s,%s.%s]," % (str(i), out_name, progression.names[i])
    out += " ]);\n\n"
    return out

out = process_enum_mapping("ToClientMsg {", "FromServer")
out += process_enum_mapping("FromClientMsg {", "ToServer")

outfile = open("./codec.js", "w")
outfile.write(out)
outfile.close()