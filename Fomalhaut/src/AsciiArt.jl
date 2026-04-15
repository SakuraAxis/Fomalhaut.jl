module AsciiArt

export print_fomalhaut_ascii_art

# ASCII art lines for Fomalhaut branding.
const ASCII_LINES = [
    "::::::::::::::     ::::    :::    :::       :::    ::::::    :::::::::::::: ",
    ":+:       +:+:+: :+:+:+  :+: :+:  :+:       :+:    :+::+:    :+:    :+:     ",
    "+:+       +:+ +:+:+ +:+ +:+   +:+ +:+       +:+    +:++:+    +:+    +:+     ",
    ":#::+::#  +#+  +:+  +#++#++:++#++:+#+       +#++:++#+++#+    +:+    +#+     ",
    "+#+       +#+       +#++#+     +#++#+       +#+    +#++#+    +#+    +#+     ",
    "#+#       #+#       #+##+#     #+##+#       #+#    #+##+#    #+#    #+#     ",
    "###       ###       ######     ################    ### ########     ###     ",
]

const ASCII_MAX_LEN = maximum(length.(ASCII_LINES))
const ASCII_C1, ASCII_C2, ASCII_C3 = [150, 50, 230], [220, 80, 130], [255, 140, 0]

# 3-point color interpolation for a smooth horizontal gradient.
function _ascii_lerp3(c1, c2, c3, t)
    t = clamp(t, 0.0, 1.0)
    f = t < 0.5 ? t * 2 : (t - 0.5) * 2
    base = t < 0.5 ? (c1, c2) : (c2, c3)
    return round.(Int, base[1] .+ (base[2] .- base[1]) .* f)
end

# Print Fomalhaut ASCII art with ANSI true-color gradient.
function print_fomalhaut_ascii_art(io::IO=stdout)
    for line in ASCII_LINES
        padded = rpad(line, ASCII_MAX_LEN)
        for (ci, ch) in enumerate(padded)
            t = ASCII_MAX_LEN > 1 ? (ci - 1) / (ASCII_MAX_LEN - 1) : 0.0
            r, g, b = _ascii_lerp3(ASCII_C1, ASCII_C2, ASCII_C3, t)
            print(io, "\e[38;2;$(r);$(g);$(b)m$(ch)")
        end
        print(io, "\e[0m\n")
    end
    return nothing
end

end # module AsciiArt
