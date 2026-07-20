const stdlib: Record<string, (args: string) => string> = {
  "print": (args) => `clx_std:print(${args})`,
  "random": (args) => `clx_std:random(${args})`
};

export default stdlib;
