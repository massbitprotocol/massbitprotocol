const path = require("path");
const execSync = require("child_process").execSync;

const srcDir = path.join(__dirname, "..");

const exec = (cmd) => {
    try {
        return execSync(cmd, { cwd: srcDir, stdio: "inherit" });
    } catch (e) {
        throw new Error(`Failed to run command \`${cmd}\``);
    }
};

exec(`yarn codegen`);
exec(`yarn build`);
exec(`yarn deploy-local`);