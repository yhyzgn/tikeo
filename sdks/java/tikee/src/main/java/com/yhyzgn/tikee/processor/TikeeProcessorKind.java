package com.yhyzgn.tikee.processor;

/** Explicit processor declaration kind used for structured worker registration. */
public enum TikeeProcessorKind {
    /** Normal SDK processor selected by job processorName. */
    SDK,
    /** Plugin processor selected by plugin processorType plus processorName. */
    PLUGIN
}
