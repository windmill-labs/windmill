"use strict";
///<reference path="../lib.deno.d.ts" />
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.resolveDns = void 0;
const dns_1 = __importDefault(require("dns"));
const resolveDns = function resolveDns(query, recordType, options) {
    if (options) {
        throw Error(`resolveDns option not implemnted yet`);
    }
    switch (recordType) {
        case "A":
        /* falls through */
        case "AAAA":
        case "CNAME":
        case "NS":
        case "PTR":
            return new Promise((resolve, reject) => {
                dns_1.default.resolve(query, recordType, (err, addresses) => {
                    if (err) {
                        reject(err);
                    }
                    else {
                        resolve(addresses);
                    }
                });
            });
        case "ANAME":
        case "CAA":
        case "MX":
        case "NAPTR":
        case "SOA":
        case "SRV":
        case "TXT":
        default:
            throw Error(`resolveDns type ${recordType} not implemnted yet`);
    }
};
exports.resolveDns = resolveDns;
