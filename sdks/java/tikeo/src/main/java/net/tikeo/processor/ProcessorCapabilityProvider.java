package net.tikeo.processor;

import java.util.List;

/**
 * Optional capability source for task processors that can advertise concrete executor names.
 */
public interface ProcessorCapabilityProvider {
    /**
     * Capabilities to add to Worker registration.
     *
     * @return capability strings such as {@code processor:demo.echo}
     */
    List<String> capabilities();
}
