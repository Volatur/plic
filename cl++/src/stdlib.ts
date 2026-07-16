const stdlib: Record<string, (args: string) => string> = {
  "io:print": (args) => `clx_std:print(${args})`,
};

export default stdlib;
