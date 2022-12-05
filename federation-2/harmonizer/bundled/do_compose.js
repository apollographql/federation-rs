Object.defineProperty(exports, "__esModule", { value: true });
try {
    const composed = bridge.composition(serviceList);
    let hints = [];
    if (composed.hints) {
        composed.hints.map((composed_hint) => {
            hints.push({ message: composed_hint.toString() });
        });
    }
    done(composed.errors
        ? { Err: composed.errors }
        : {
            Ok: {
                supergraphSdl: composed.supergraphSdl,
                hints,
            },
        });
}
catch (err) {
    done({ Err: [{ message: err.toString() }] });
}
//# sourceMappingURL=do_compose.js.map