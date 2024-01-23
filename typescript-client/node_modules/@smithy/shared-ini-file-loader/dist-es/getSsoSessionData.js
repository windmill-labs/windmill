import { IniSectionType } from "@smithy/types";
import { CONFIG_PREFIX_SEPARATOR } from "./loadSharedConfigFiles";
export const getSsoSessionData = (data) => Object.entries(data)
    .filter(([key]) => key.startsWith(IniSectionType.SSO_SESSION + CONFIG_PREFIX_SEPARATOR))
    .reduce((acc, [key, value]) => ({ ...acc, [key.split(CONFIG_PREFIX_SEPARATOR)[1]]: value }), {});
