import sys
import json

next_argument_category = None

arg_position = 0
kodi_config_path = None
requested_plugin_path = None
output_file = None
for arg in sys.argv[1:]:
    if next_argument_category == "path":
        next_argument_category = None
        sys.path.append(arg)
    elif next_argument_category == None:
        if arg == "-P":
            next_argument_category = "path"
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

print("kodidl: requesting for {}".format(requested_plugin_path))
kodi = xbmcemu.KodiInstance(kodi_config_path)
result = kodi.run_url(requested_plugin_path)
print("kodidl: got as result:")
result.pretty_print("kodidl: ")
print("kodidl: saving...")
dumped = json.dumps(result.to_dict())
f = open(output_file, "w")
f.write(dumped)
f.close()
print("kodidl: finished")
