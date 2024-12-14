#pragma once

#define assert(ignore) ((void)0)
#define static_assert(cnd, msg) assert(cnd && msg)
