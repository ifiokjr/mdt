/**
 * Define a configuration for the mdt processor. This is loaded by the
 * **lsp** and **cli**.
 */
export function defineConfig(
	config: WrappedConfig<MdtConfig>,
): Promise<MdtConfig> {
	let value: Promise<MdtConfig>;
	if (typeof config === "function") {
		value = Promise.resolve(config());
	} else {
		value = Promise.resolve(config);
	}

	return value;
}

export interface MdtConfig {
	data: Record<string, unknown>;
}

export type WrappedConfig<C> = C | (() => C) | Promise<C> | (() => Promise<C>);
