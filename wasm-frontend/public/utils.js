const params = new URLSearchParams(window.location.search);
var code = params.get("code");
history.replaceState(null, "", window.location.href.replace(window.location.search, ""));

import { with_assets } from '/pkg/wasm_frontend.js';

export function load_asset(asset){
	// Necessary to gain an async runtime
	let execute = async () => {
		try {
			// TODO: Use Promise.all() here
			for (const el of asset.entries()) {
				let response = await fetch(el[1]);
				
				let buffer = await response.arrayBuffer();
				asset.set(el[0], new Uint8Array(buffer));
			}
			
		} catch (e) {
			console.error("Error fetching assets: " + e);
			document.getElementById("errorreason").innerText = "Error fetching assets: " + e;
			return;
		}
		with_assets(asset);
	};

	
	execute();
}
export default load_asset;
