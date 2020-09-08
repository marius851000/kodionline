import sys
import json

next_argument_category = None

arg_position = 0
kodi_config_path = None
requested_plugin_path = None
output_file = None
expected_input = []

special_data = {
    "language_order": [],
    "resolution_order": [],
    "format_order": [],
}

list_key = None
for arg in sys.argv[1:]:
    if next_argument_category == "path":
        sys.path.append(arg)
        next_argument_category = None
    elif next_argument_category == "expected_input":
        expected_input.append(arg)
        next_argument_category = None
    elif next_argument_category == "special_add_list_key":
        list_key = arg
        if list_key not in special_data:
            special_data[list_key] = []
        next_argument_category = "special_add_list_value"
    elif next_argument_category == "special_add_list_value":
        list_value = arg
        special_data[list_key].append(list_value)
        next_argument_category = None
    elif next_argument_category == None:
        if arg == "-P":
            next_argument_category = "path"
        elif arg == "-I":
            next_argument_category = "expected_input"
        elif arg == "-AL":
            next_argument_category = "special_add_list_key"
        elif arg[0] == "-":
            raise BaseException("unknown argument: {}".format(arg))
        else:
            if arg_position == 0:
                kodi_config_path = arg
            elif arg_position == 1:
                requested_plugin_path = arg
            elif arg_position == 2:
                output_file = arg
            else:
                raise BaseException("too much unnamed argument used")
            arg_position += 1

    else:
        raise BaseException("next_argument_category is invalid: {}".format(next_argument_category))

import xbmcemu

print("kodi---------------------------")

print("kodidl: requesting for {}".format(requested_plugin_path))
if len(expected_input) > 0:
    print("kodidl: with inputs {}".format(expected_input))

kodi = xbmcemu.KodiInstance(kodi_config_path)
kodi.planned_input = expected_input
kodi.additional_input = special_data
try:
    result = kodi.run_url(requested_plugin_path)
    print("kodidl: got as result:")
    result.pretty_print("kodidl: ")
    print("kodidl: saving...")
    out_dic = result.to_dict()
    out_dic["type"] = "Content"

except xbmcemu.exception.KeyboardInputRequired as keyboard_exception:
    keyboard = keyboard_exception.keyboard
    out_dic = {
        "default": keyboard.text,
        "heading": keyboard.heading,
        "hidden": keyboard.hidden,
    }
    out_dic["type"] = "Keyboard"

f = open(output_file, "w")
f.write(json.dumps(out_dic))
f.close()
print("kodidl: finished")
sys.stdout.flush()
sys.stderr.flush()
