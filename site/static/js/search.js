/**
 * Search and Autocomplete Functionality for Janitor
 * Provides typeahead search for packages, codebases, and other entities
 */

(function(window, document, $) {
    'use strict';

    // Ensure Janitor namespace exists
    window.Janitor = window.Janitor || {};

    /**
     * Search Module
     */
    Janitor.Search = (function() {
        
        // Configuration
        var config = {
            endpoints: {
                packages: '/api/pkgnames',
                codebases: '/api/codebases',
                campaigns: '/api/campaigns'
            },
            cache: {
                enabled: true,
                ttl: 300000, // 5 minutes
                maxSize: 1000
            },
            typeahead: {
                minLength: 2,
                highlight: true,
                hint: true,
                classNames: {
                    menu: 'typeahead-menu',
                    dataset: 'typeahead-dataset',
                    suggestion: 'typeahead-suggestion'
                }
            }
        };

        // Cache for search results
        var cache = new Map();
        var cacheTimestamps = new Map();

        function isValidCacheEntry(key) {
            if (!config.cache.enabled) return false;
            
            var timestamp = cacheTimestamps.get(key);
            if (!timestamp) return false;
            
            return Date.now() - timestamp < config.cache.ttl;
        }

        function setCacheEntry(key, data) {
            if (!config.cache.enabled) return;
            
            // Prune cache if it's getting too large
            if (cache.size >= config.cache.maxSize) {
                var oldestKey = cache.keys().next().value;
                cache.delete(oldestKey);
                cacheTimestamps.delete(oldestKey);
            }
            
            cache.set(key, data);
            cacheTimestamps.set(key, Date.now());
        }

        function getCacheEntry(key) {
            if (!config.cache.enabled || !isValidCacheEntry(key)) {
                return null;
            }
            return cache.get(key);
        }

        /**
         * Bloodhound source for typeahead suggestions
         */
        function createBloodhoundSource(endpoint, transform) {
            return new Bloodhound({
                datumTokenizer: function(datum) {
                    return Bloodhound.tokenizers.whitespace(datum.name || datum.value || datum);
                },
                queryTokenizer: Bloodhound.tokenizers.whitespace,
                remote: {
                    url: endpoint + '?q=%QUERY',
                    wildcard: '%QUERY',
                    transform: transform || function(response) {
                        return response.data || response;
                    },
                    transport: function(opts, onSuccess, onError) {
                        var cacheKey = opts.url;
                        var cached = getCacheEntry(cacheKey);
                        
                        if (cached) {
                            onSuccess(cached);
                            return;
                        }
                        
                        return Janitor.Ajax.get(opts.url)
                            .done(function(data) {
                                var transformed = opts.transform ? opts.transform(data) : data;
                                setCacheEntry(cacheKey, transformed);
                                onSuccess(transformed);
                            })
                            .fail(function(xhr, status, error) {
                                console.error('Typeahead remote request failed:', error);
                                onError();
                            });
                    }
                }
            });
        }

        /**
         * Initialize package name typeahead
         */
        function initPackageTypeahead(selector, options) {
            var opts = $.extend({}, config.typeahead, options);
            
            var packages = createBloodhoundSource(config.endpoints.packages, function(response) {
                // Transform package response to typeahead format
                if (Array.isArray(response)) {
                    return response.map(function(pkg) {
                        return typeof pkg === 'string' ? {name: pkg, value: pkg} : pkg;
                    });
                }
                return response.data || [];
            });

            packages.initialize();

            $(selector).typeahead(opts, {
                name: 'packages',
                display: 'name',
                source: packages.ttAdapter(),
                templates: {
                    header: '<h5 class="typeahead-header">Packages</h5>',
                    suggestion: function(data) {
                        return '<div class="typeahead-suggestion">' +
                               '<strong>' + data.name + '</strong>' +
                               (data.description ? '<br><small>' + data.description + '</small>' : '') +
                               '</div>';
                    },
                    notFound: '<div class="typeahead-empty">No packages found</div>'
                }
            });

            // Handle selection
            $(selector).on('typeahead:select', function(event, suggestion) {
                if (options && options.onSelect) {
                    options.onSelect(suggestion);
                } else {
                    // Default behavior: navigate to package page
                    window.location.href = '/pkg/' + encodeURIComponent(suggestion.name);
                }
            });
        }

        /**
         * Initialize codebase typeahead
         */
        function initCodebaseTypeahead(selector, options) {
            var opts = $.extend({}, config.typeahead, options);
            
            var codebases = createBloodhoundSource(config.endpoints.codebases, function(response) {
                return response.data || [];
            });

            codebases.initialize();

            $(selector).typeahead(opts, {
                name: 'codebases',
                display: 'name',
                source: codebases.ttAdapter(),
                templates: {
                    header: '<h5 class="typeahead-header">Codebases</h5>',
                    suggestion: function(data) {
                        return '<div class="typeahead-suggestion">' +
                               '<strong>' + data.name + '</strong>' +
                               (data.branch ? '<br><small>Branch: ' + data.branch + '</small>' : '') +
                               (data.vcs_type ? '<small class="text-muted"> (' + data.vcs_type + ')</small>' : '') +
                               '</div>';
                    },
                    notFound: '<div class="typeahead-empty">No codebases found</div>'
                }
            });

            $(selector).on('typeahead:select', function(event, suggestion) {
                if (options && options.onSelect) {
                    options.onSelect(suggestion);
                } else {
                    window.location.href = '/codebase/' + encodeURIComponent(suggestion.name);
                }
            });
        }

        /**
         * Initialize search form with typeahead
         */
        function initSearchForm(selector, options) {
            var $form = $(selector);
            var $input = $form.find('input[type="search"], input[name="q"]');
            
            if ($input.length === 0) {
                console.warn('No search input found in form:', selector);
                return;
            }

            // Determine search type based on form context or options
            var searchType = options && options.type;
            if (!searchType) {
                // Try to infer from form attributes or URL
                var action = $form.attr('action') || '';
                if (action.includes('pkg') || action.includes('package')) {
                    searchType = 'packages';
                } else if (action.includes('codebase')) {
                    searchType = 'codebases';
                } else {
                    searchType = 'packages'; // Default
                }
            }

            // Initialize appropriate typeahead
            switch (searchType) {
                case 'packages':
                    initPackageTypeahead($input, options);
                    break;
                case 'codebases':
                    initCodebaseTypeahead($input, options);
                    break;
                default:
                    console.warn('Unknown search type:', searchType);
            }

            // Handle form submission
            $form.on('submit', function(e) {
                var value = $input.val().trim();
                if (!value) {
                    e.preventDefault();
                    return false;
                }
                
                if (options && options.onSubmit) {
                    var result = options.onSubmit(value, searchType);
                    if (result === false) {
                        e.preventDefault();
                        return false;
                    }
                }
            });
        }

        /**
         * Initialize global search functionality
         */
        function initGlobalSearch() {
            // Main search forms
            initSearchForm('.search-form', {type: 'packages'});
            initSearchForm('#package-search-form', {type: 'packages'});
            initSearchForm('#codebase-search-form', {type: 'codebases'});
            
            // Search inputs in sidebars and navigation
            initPackageTypeahead('.package-search-input');
            initCodebaseTypeahead('.codebase-search-input');
            
            // Generic search inputs (infer type from context)
            $('input[data-search-type]').each(function() {
                var $input = $(this);
                var searchType = $input.data('search-type');
                var options = {
                    type: searchType,
                    onSelect: function(suggestion) {
                        var url = $input.data('result-url');
                        if (url) {
                            url = url.replace('{name}', encodeURIComponent(suggestion.name));
                            window.location.href = url;
                        }
                    }
                };
                
                switch (searchType) {
                    case 'packages':
                        initPackageTypeahead($input, options);
                        break;
                    case 'codebases':
                        initCodebaseTypeahead($input, options);
                        break;
                }
            });
        }

        /**
         * Clear all caches
         */
        function clearCache() {
            cache.clear();
            cacheTimestamps.clear();
        }

        // Initialize when DOM is ready
        $(document).ready(function() {
            initGlobalSearch();
        });

        // Public API
        return {
            initPackageTypeahead: initPackageTypeahead,
            initCodebaseTypeahead: initCodebaseTypeahead,
            initSearchForm: initSearchForm,
            initGlobalSearch: initGlobalSearch,
            clearCache: clearCache,
            config: config
        };
    })();

})(window, document, jQuery);